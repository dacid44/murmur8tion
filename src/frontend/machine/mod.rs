use std::{collections::VecDeque, time::Instant};

use arbitrary_int::u4;
use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    render::render_resource::Extent3d,
};
use image::RgbaImage;
use keymap::KeyMapping;

use crate::{
    hardware::{self, Chip8, DynamicMachine, KeyEvent, Machine as HardwareMachine},
    model::{DynamicModel, Model},
    screen::Screen,
};

use super::{audio::Chip8Audio, rom::Rom, EmulatorData, EmulatorEvent, Frame};

mod keymap;

pub const FRAME_TICK_TIME: DiagnosticPath = DiagnosticPath::const_new("frame_tick_time");

#[derive(Resource)]
pub struct Machine {
    pub machine: DynamicMachine,
    // pub machine: Box<dyn hardware::Machine>,
    queued_inputs: Vec<(u4, KeyEvent)>,
}

impl Machine {
    fn new(model: DynamicModel, rom: &[u8]) -> Self {
        Self {
            machine: DynamicMachine::new(model, rom),
            // machine: model.into_dyn_machine(rom),
            queued_inputs: Vec::new(),
        }
    }
}

pub fn machine_plugin(app: &mut App) {
    app.init_resource::<KeyMapping>()
        .register_diagnostic(Diagnostic::new(FRAME_TICK_TIME))
        .add_systems(
            PostUpdate,
            handle_ui_events.run_if(on_event::<EmulatorEvent>),
        )
        .add_systems(FixedPreUpdate, handle_machine_input)
        .add_systems(
            FixedUpdate,
            machine_system.run_if(resource_exists::<Machine>),
        )
        .add_systems(FixedPostUpdate, machine_audio);
}

pub fn handle_ui_events(
    mut commands: Commands,
    mut ui_data: ResMut<EmulatorData>,
    mut ui_events: EventReader<EmulatorEvent>,
    mut time: ResMut<Time<Fixed>>,
    rom: Option<Res<Rom>>,
) {
    for event in ui_events.read() {
        match event {
            EmulatorEvent::ResetMachine => {
                if let Some(rom) = rom.as_ref() {
                    if ui_data.use_default_framerate {
                        let rate = ui_data.machine_model.default_framerate();
                        ui_data.frame_rate = rate;
                        time.set_timestep_hz(rate);
                    }
                    commands.insert_resource(Machine::new(ui_data.machine_model.clone(), &rom.0))
                }
            }
            EmulatorEvent::ChangeTickRate(rate) => time.set_timestep_hz(*rate),
            _ => {}
        }
    }
}

fn handle_machine_input(
    machine: Option<ResMut<Machine>>,
    key_mapping: Res<KeyMapping>,
    mut key_events: EventReader<KeyboardInput>,
) {
    let inputs = key_events.read();
    if let Some(mut machine) = machine {
        machine.queued_inputs.extend(inputs.filter_map(|event| {
            key_mapping.keys.get(&event.key_code).map(|key| {
                (
                    *key,
                    match event.state {
                        ButtonState::Pressed => KeyEvent::Press,
                        ButtonState::Released => KeyEvent::Release,
                    },
                )
            })
        }));
    }
}

fn machine_system(
    mut commands: Commands,
    mut machine: ResMut<Machine>,
    ui_data: Res<EmulatorData>,
    mut frame: ResMut<Frame>,
    mut images: ResMut<Assets<Image>>,
    mut diagnostics: Diagnostics,
) {
    if ui_data.paused {
        return;
    }

    let t0 = Instant::now();

    let image = images
        .get_mut(&frame.handle)
        .expect("Emulator frame not found");
    frame.size = write_frame(image, machine.machine.render_frame(&ui_data.palette));

    let Machine {
        machine,
        queued_inputs,
    } = machine.as_mut();
    for (key, event) in queued_inputs.drain(..) {
        machine.event(key, event);
    }
    if let Err(error) = machine.tick_many(ui_data.cycles_per_frame) {
        error!("Emulator error: {error}");
        frame.size = write_frame(image, machine.render_frame(&ui_data.palette));
        commands.remove_resource::<Machine>();
        return;
    }

    let t1 = Instant::now();
    diagnostics.add_measurement(&FRAME_TICK_TIME, || (t1 - t0).as_millis() as f64);
}

fn machine_audio(
    machine: Option<Res<Machine>>,
    ui_data: Res<EmulatorData>,
    mut audio: ResMut<Chip8Audio>,
) {
    match (
        machine
            .as_ref()
            .filter(|machine| machine.machine.sound_active()),
        ui_data.paused,
    ) {
        (Some(machine), false) => audio.render_audio(
            machine.machine.pitch(),
            *machine.machine.audio_pattern(),
            1.0 / ui_data.frame_rate,
        ),
        // Don't reset if machine exists but is paused
        (Some(_), true) => {}
        (None, _) => audio.reset(),
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
