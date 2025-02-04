use std::collections::VecDeque;

use arbitrary_int::u4;
use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    render::render_resource::Extent3d,
};
use image::RgbaImage;
use keymap::KeyMapping;

use crate::{
    hardware::{DynamicMachine, KeyEvent},
    model::{DynamicModel, Model},
};

use super::{audio::Chip8Audio, rom::Rom, EmulatorData, EmulatorEvent, Frame};

mod keymap;

#[derive(Resource)]
pub struct Machine {
    pub machine: DynamicMachine,
    queued_inputs: VecDeque<(u4, KeyEvent)>,
}

impl Machine {
    fn new(model: &DynamicModel, rom: &[u8]) -> Self {
        Self {
            machine: DynamicMachine::new(model, rom),
            queued_inputs: VecDeque::new(),
        }
    }
}

pub fn machine_plugin(app: &mut App) {
    app.init_resource::<KeyMapping>()
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
                    commands.insert_resource(Machine::new(&ui_data.machine_model, &rom.0))
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
) {
    if ui_data.paused {
        return;
    }

    let image = images
        .get_mut(&frame.handle)
        .expect("Emulator frame not found");
    frame.size = write_frame(image, machine.machine.render_frame(&ui_data.palette));

    for _ in 0..(ui_data.cycles_per_frame) {
        if let Some((key, event)) = machine.queued_inputs.pop_front() {
            machine.machine.event(key, event);
        }
        if let Err(error) = machine.machine.tick() {
            error!("Emulator error: {error}");
            frame.size = write_frame(image, machine.machine.render_frame(&ui_data.palette));
            commands.remove_resource::<Machine>();
            return;
        }
    }
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
