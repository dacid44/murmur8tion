use std::{collections::VecDeque, time::Duration};

use bevy::{
    color::palettes::css,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    tasks::{block_on, poll_once, IoTaskPool, Task},
    utils::HashMap,
    winit::WinitSettings,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use hardware::{Chip8, KeyEvent};
use ux::u4;

mod hardware;
mod instruction;
mod model;
mod screen;

#[derive(Resource)]
struct EmulatorFrame {
    frame_handle: Handle<Image>,
    frame_sprite: Entity,
}

#[derive(Resource)]
struct Machine {
    machine: Chip8<model::CosmacVip>,
    queued_inputs: VecDeque<(u4, KeyEvent)>,
}

impl Machine {
    fn new(model: model::CosmacVip, rom: &[u8]) -> Self {
        Self {
            machine: Chip8::new(model, rom),
            queued_inputs: VecDeque::new(),
        }
    }
}

#[derive(Resource)]
struct Rom(Vec<u8>);

#[derive(Component)]
struct PickRom(Task<Option<(String, Vec<u8>)>>);

#[derive(Resource)]
struct UiData {
    paused: bool,
    frame_rate: f64,
    cycles_per_frame: u32,
    actual_fps: f64,
    rom_name: String,
}

impl Default for UiData {
    fn default() -> Self {
        Self {
            paused: false,
            frame_rate: 60.0,
            cycles_per_frame: 100,
            actual_fps: 0.0,
            rom_name: "No ROM loaded".to_owned(),
        }
    }
}

#[derive(Event)]
enum UiEvent {
    PickRom,
    ResetMachine,
    ChangeTickRate(f64),
    ResizeFrame(Rect),
}

#[derive(Resource)]
struct KeyMapping {
    keys: HashMap<KeyCode, u4>,
}

const DEFAULT_KEY_MAPPING: [KeyCode; 16] = [
    KeyCode::KeyX,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::KeyQ,
    KeyCode::KeyW,
    KeyCode::KeyE,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyZ,
    KeyCode::KeyC,
    KeyCode::Digit4,
    KeyCode::KeyR,
    KeyCode::KeyF,
    KeyCode::KeyV,
];

impl Default for KeyMapping {
    fn default() -> Self {
        Self {
            keys: DEFAULT_KEY_MAPPING
                .iter()
                .enumerate()
                .map(|(i, key)| (*key, i.try_into().unwrap()))
                .collect(),
        }
    }
}

fn main() {
    println!("Hello, world!");

    App::new()
        .insert_resource(WinitSettings::game())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<UiData>()
        .init_resource::<KeyMapping>()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "CHIP-8 Emulator".to_owned(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_plugins(EguiPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_event::<UiEvent>()
        .add_systems(Startup, setup_system)
        .add_systems(PreUpdate, update_ui_data)
        .add_systems(
            Update,
            (
                emulator_ui_system,
                rom_loaded.run_if(any_with_component::<PickRom>),
            ),
        )
        .add_systems(PostUpdate, handle_ui_events)
        .add_systems(FixedPreUpdate, handle_machine_input)
        .add_systems(
            FixedUpdate,
            machine_system.run_if(resource_exists::<Machine>),
        )
        .add_systems(FixedPostUpdate, machine_audio)
        .run();
}

fn setup_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut pitch_assets: ResMut<Assets<Pitch>>,
) {
    commands.spawn(Camera2d);

    let image = Image::new_fill(
        Extent3d {
            width: screen::CosmacVipScreen::WIDTH as u32,
            height: screen::CosmacVipScreen::HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &(css::BEIGE.to_u8_array()),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(image);

    let sprite = commands.spawn(Sprite::from_image(handle.clone())).id();
    let pitch = pitch_assets.add(Pitch::new(261.63, Duration::MAX));
    commands.insert_resource(EmulatorFrame {
        frame_handle: handle,
        frame_sprite: sprite,
    });
    commands.spawn((AudioPlayer(pitch), PlaybackSettings::LOOP.paused()));
}

fn update_ui_data(
    mut ui_data: ResMut<UiData>,
    time: Res<Time<Fixed>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    ui_data.frame_rate = 1.0 / time.timestep().as_secs_f64();
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    {
        ui_data.actual_fps = fps;
    }
}

fn emulator_ui_system(
    mut contexts: EguiContexts,
    mut ui_data: ResMut<UiData>,
    mut ui_events: EventWriter<UiEvent>,
    mut last_available_rect: Local<Rect>,
) {
    let egui_context = contexts.ctx_mut();
    egui::SidePanel::new(egui::panel::Side::Left, "control-panel").show(egui_context, |ui| {
        ui.label(format!("FPS: {}", ui_data.actual_fps));

        egui::Frame::group(&egui_context.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&ui_data.rom_name);
                if ui.button("Choose ROM").clicked() {
                    ui_events.send(UiEvent::PickRom);
                }
            });

            if ui.button("Reset Emulator").clicked() {
                ui_events.send(UiEvent::ResetMachine);
            }
        });

        egui::Frame::group(&egui_context.style()).show(ui, |ui| {
            ui.toggle_value(&mut ui_data.paused, "Pause");

            let original_tick_rate = ui_data.frame_rate;
            ui.add(
                egui::Slider::new(&mut ui_data.frame_rate, 0.5..=120.0)
                    .step_by(0.5)
                    .text("Target FPS"),
            );
            if ui_data.frame_rate != original_tick_rate {
                ui_events.send(UiEvent::ChangeTickRate(ui_data.frame_rate));
            }

            ui.add(
                egui::Slider::new(&mut ui_data.cycles_per_frame, 1..=1000)
                    .logarithmic(true)
                    .text("Cycles per frame"),
            );
        });

        ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover())
    });

    let available_rect = egui_to_bevy_rect(contexts.ctx_mut().available_rect());
    if available_rect != *last_available_rect {
        *last_available_rect = available_rect;
        ui_events.send(UiEvent::ResizeFrame(available_rect));
    }
}

fn handle_ui_events(
    mut ui_events: EventReader<UiEvent>,
    mut commands: Commands,
    mut time: ResMut<Time<Fixed>>,
    rom: Option<Res<Rom>>,
    emulator_output: Res<EmulatorFrame>,
    mut frame_sprite_query: Query<(&mut Sprite, &mut Transform)>,
) {
    for event in ui_events.read() {
        match event {
            UiEvent::PickRom => {
                commands.spawn(pick_rom());
            }
            UiEvent::ResetMachine => {
                if let Some(rom) = rom.as_ref() {
                    commands.insert_resource(Machine::new(model::CosmacVip, &rom.0))
                }
            }
            UiEvent::ChangeTickRate(rate) => time.set_timestep_hz(*rate),
            UiEvent::ResizeFrame(rect) => {
                let (mut sprite, mut transform) = frame_sprite_query
                    .get_mut(emulator_output.frame_sprite)
                    .expect("Could not find frame entity");

                let frame_size = Vec2::new(
                    screen::CosmacVipScreen::WIDTH as f32,
                    screen::CosmacVipScreen::HEIGHT as f32,
                );
                let scale = (rect.size() / frame_size).min_element();
                sprite.custom_size = Some(frame_size * scale);
                transform.translation.x = rect.min.x / 2.0;
            }
        }
    }
}

fn pick_rom() -> PickRom {
    let task = IoTaskPool::get().spawn(async {
        let file = rfd::AsyncFileDialog::new()
            .set_title("Choose a ROM file")
            .add_filter("Chip-8 ROMs", &["ch8"])
            .pick_file()
            .await?;

        match async_fs::read(file.path()).await {
            Ok(data) => Some((
                file.path()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "..".to_owned()),
                data,
            )),
            Err(error) => {
                error!(
                    "Error reading chosen file {}: {}",
                    file.path().display(),
                    error
                );
                None
            }
        }
    });
    PickRom(task)
}

fn rom_loaded(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut PickRom)>,
    mut ui_data: ResMut<UiData>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(Some(rom)) = block_on(poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            ui_data.rom_name = rom.0;
            commands.insert_resource(Rom(rom.1));
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
    ui_data: Res<UiData>,
    emulator_output: Res<EmulatorFrame>,
    mut images: ResMut<Assets<Image>>,
) {
    if ui_data.paused {
        return;
    }

    let image = images
        .get_mut(&emulator_output.frame_handle)
        .expect("Emulator frame not found");
    image.data = machine.machine.render_frame().into_vec();

    for _ in 0..(ui_data.cycles_per_frame) {
        if let Some((key, event)) = machine.queued_inputs.pop_front() {
            machine.machine.event(key, event);
        }
        if let Err(error) = machine.machine.tick() {
            error!("Emulator error: {error}");
            image.data = machine.machine.render_frame().into_vec();
            commands.remove_resource::<Machine>();
            return;
        }
    }
}

fn machine_audio(
    machine: Option<Res<Machine>>,
    ui_data: Res<UiData>,
    beep: Query<&AudioSink, With<AudioPlayer<Pitch>>>,
) {
    let is_machine_sound_active = machine.is_some_and(|machine| machine.machine.sound_active());
    let beep = beep.single();
    if beep.is_paused() == (is_machine_sound_active && !ui_data.paused) {
        beep.toggle();
    }
}

fn egui_to_bevy_rect(rect: egui::Rect) -> Rect {
    Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y)
}
