use std::time::{Duration, Instant};

use arbitrary_int::u4;
use async_channel::{Receiver, Sender};
use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    render::render_resource::Extent3d,
};
use image::RgbaImage;
use keymap::KeyMapping;

use crate::{
    hardware::{self, DynamicMachine, KeyEvent, Machine as HardwareMachine},
    model::{CosmacVip, Model},
};

use super::{audio::Chip8Audio, rom::Rom, EmulatorData, EmulatorEvent, Frame};

mod keymap;

pub const FRAME_TICK_TIME: DiagnosticPath = DiagnosticPath::const_new("frame_tick_time");
pub const EMULATOR_FPS: DiagnosticPath = DiagnosticPath::const_new("emulator_fps");

#[derive(Resource)]
pub struct Machine {
    initialized: bool,
    pub machine: DynamicMachine,
    pub tx: Sender<ToMachine>,
    frame_rx: Receiver<FrameEvent>,
}

pub enum ToMachine {
    Input(u4, KeyEvent),
    ResetMachine(DynamicMachine),
    Pause(bool),
    Step,
    SetFrequency(f64),
    SetIpf(u32),
    Exit,
}

struct FrameEvent {
    machine: Option<DynamicMachine>,
    error: Option<hardware::Error>,
    frame_time: Duration,
    audio_status: AudioStatus,
}

enum AudioStatus {
    Play(Duration),
    Paused,
    Reset,
}

pub fn machine_plugin(app: &mut App) {
    app.init_resource::<KeyMapping>()
        .register_diagnostic(Diagnostic::new(FRAME_TICK_TIME))
        .register_diagnostic(Diagnostic::new(EMULATOR_FPS))
        .add_systems(Startup, setup)
        .add_systems(Update, handle_machine.pipe(render_machine_output))
        .add_systems(PostUpdate, handle_ui_events);
    // .add_systems(FixedPreUpdate, handle_machine_input)
    // .add_systems(
    //     FixedUpdate,
    //     machine_system.run_if(resource_exists::<Machine>),
    // )
    // .add_systems(FixedPostUpdate, machine_audio);
}

fn handle_ui_events(
    mut ui_data: ResMut<EmulatorData>,
    mut last_ui_data: Local<EmulatorData>,
    mut ui_events: EventReader<EmulatorEvent>,
    rom: Option<Res<Rom>>,
    machine: ResMut<Machine>,
) {
    if ui_data.paused != last_ui_data.paused {
        machine
            .tx
            .try_send(ToMachine::Pause(ui_data.paused))
            .unwrap();
    }
    if ui_data.cycles_per_frame != last_ui_data.cycles_per_frame {
        machine
            .tx
            .try_send(ToMachine::SetIpf(ui_data.cycles_per_frame))
            .unwrap();
    }
    if ui_data.frame_rate != last_ui_data.frame_rate {
        machine
            .tx
            .try_send(ToMachine::SetFrequency(ui_data.frame_rate))
            .unwrap();
    }

    for event in ui_events.read() {
        match event {
            EmulatorEvent::ResetMachine => {
                if let Some(rom) = rom.as_ref() {
                    if ui_data.use_default_framerate {
                        let rate = ui_data.machine_model.default_framerate();
                        ui_data.frame_rate = rate;
                        machine
                            .tx
                            .try_send(ToMachine::SetFrequency(ui_data.frame_rate))
                            .unwrap();
                    }
                    machine
                        .tx
                        .try_send(ToMachine::ResetMachine(DynamicMachine::new(
                            ui_data.machine_model.clone(),
                            &rom.0,
                        )))
                        .unwrap();
                }
            }
            _ => {}
        }
    }

    *last_ui_data = ui_data.as_ref().clone();
}

fn setup(mut commands: Commands, emulator_data: Res<EmulatorData>) {
    let (tx, frame_rx) =
        spawn_machine_thread(emulator_data.frame_rate, emulator_data.cycles_per_frame);
    commands.insert_resource(Machine {
        initialized: false,
        machine: DynamicMachine::new_cosmac_vip(CosmacVip::default(), &[]),
        tx,
        frame_rx,
    });
}

fn spawn_machine_thread(frequency: f64, ipf: u32) -> (Sender<ToMachine>, Receiver<FrameEvent>) {
    let (tx, rx) = async_channel::unbounded();
    let (frame_tx, frame_rx) = async_channel::unbounded();
    std::thread::spawn(move || {
        let mut machine = None;
        let mut error = None;
        let mut paused = false;
        let mut timestep = Duration::from_secs_f64(1.0 / frequency);
        let mut ipf = ipf;
        let mut ts = Instant::now();
        let mut last_frame = ts;
        'outer: loop {
            let now = Instant::now();
            let frame_time = now - last_frame;
            last_frame = now;
            frame_tx
                .try_send(FrameEvent {
                    machine: machine.clone(),
                    error: error.as_ref().filter(|_| machine.is_some()).cloned(),
                    frame_time,
                    audio_status: match (
                        machine
                            .as_ref()
                            .is_some_and(|machine| machine.sound_active()),
                        paused,
                    ) {
                        (true, false) => AudioStatus::Play(timestep),
                        (true, true) => AudioStatus::Paused,
                        _ => AudioStatus::Reset,
                    },
                })
                .expect("Failed to send frame, receiver disconnected");

            if error.is_some() {
                machine = None;
            }

            let mut inputs = Vec::new();
            while let Ok(message) = rx.try_recv() {
                match message {
                    ToMachine::Input(key, event) => inputs.push((key, event)),
                    ToMachine::ResetMachine(new_machine) => {
                        machine = Some(new_machine);
                        error = None;
                    }
                    ToMachine::Pause(pause) => paused = pause,
                    ToMachine::Step => {
                        if let Some(machine) =
                            machine.as_mut().filter(|_| error.is_none())
                        {
                            println!("stepping");
                            if let Err(err) = machine.tick() {
                                error = Some(err);
                            }
                        }
                    }
                    ToMachine::SetFrequency(frequency) => {
                        timestep = Duration::from_secs_f64(1.0 / frequency)
                    }
                    ToMachine::SetIpf(new_ipf) => ipf = new_ipf,
                    ToMachine::Exit => break 'outer,
                }
            }

            if let Some(machine) = machine.as_mut().filter(|_| !paused && error.is_none()) {
                for (key, event) in inputs {
                    machine.event(key, event);
                }
                machine.tick_timers();
                if let Err(err) = machine.tick_many(ipf) {
                    error = Some(err);
                }
            }

            let now = Instant::now();
            while ts <= now {
                ts += timestep;
            }
            spin_sleep::sleep_until(ts);
        }
    });
    (tx, frame_rx)
}

fn handle_machine(
    mut machine: ResMut<Machine>,
    key_mapping: Res<KeyMapping>,
    mut key_events: EventReader<KeyboardInput>,
    mut diagnostics: Diagnostics,
    exit: EventReader<AppExit>,
) -> Vec<(AudioStatus, u8, [u8; 16])> {
    for (key, event) in key_events.read().filter_map(|event| {
        key_mapping.keys.get(&event.key_code).map(|key| {
            (
                *key,
                match event.state {
                    ButtonState::Pressed => KeyEvent::Press,
                    ButtonState::Released => KeyEvent::Release,
                },
            )
        })
    }) {
        machine.tx.try_send(ToMachine::Input(key, event)).unwrap();
    }
    if !exit.is_empty() {
        machine.tx.try_send(ToMachine::Exit).unwrap();
    }

    let mut machine_audio = Vec::new();
    while let Ok(event) = machine.frame_rx.try_recv() {
        if let Some(event_machine) = event.machine {
            machine.initialized = true;
            machine.machine = event_machine;
        }
        if let Some(error) = event.error {
            error!("Emulator error: {error}");
        }
        machine_audio.push((
            event.audio_status,
            machine.machine.pitch(),
            *machine.machine.audio_pattern(),
        ));
        if machine.initialized {
            diagnostics.add_measurement(&EMULATOR_FPS, || 1.0 / event.frame_time.as_secs_f64());
        }
    }
    machine_audio
}

fn render_machine_output(
    machine_audio: In<Vec<(AudioStatus, u8, [u8; 16])>>,
    machine: Res<Machine>,
    emulator_data: Res<EmulatorData>,
    mut frame: ResMut<Frame>,
    mut images: ResMut<Assets<Image>>,
    mut audio: ResMut<Chip8Audio>,
) {
    if machine.initialized {
        let image = images
            .get_mut(&frame.handle)
            .expect("Emulator frame not found");
        frame.size = write_frame(image, machine.machine.render_frame(&emulator_data.palette));
    }

    for (status, pitch, pattern) in machine_audio.0 {
        match status {
            AudioStatus::Play(timestep) => {
                audio.render_audio(pitch, pattern, timestep.as_secs_f64())
            }
            AudioStatus::Paused => {}
            AudioStatus::Reset => audio.reset(),
        }
    }
}

fn write_frame(texture: &mut Image, frame: RgbaImage) -> UVec2 {
    if texture.width() != frame.width() || texture.height() != texture.height() {
        texture.resize(Extent3d {
            width: frame.width(),
            height: frame.height(),
            depth_or_array_layers: 1,
        });
    }
    texture.data = frame.into_vec();
    texture.size()
}
