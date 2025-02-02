use std::{collections::VecDeque, time::Duration};

use audio::Chip8Audio;
use bevy::{
    audio::AddAudioSource,
    color::palettes::css,
    diagnostic::{
        Diagnostic, DiagnosticPath, DiagnosticsStore, FrameTimeDiagnosticsPlugin,
        RegisterDiagnostic,
    },
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
use hardware::{DynamicMachine, KeyEvent};
use image::RgbaImage;
use model::{DynamicModel, Model};
use screen::Palette;
use ux::u4;
use widgets::{model_selector, palette_editor};

mod audio;
mod hardware;
mod instruction;
mod model;
mod screen;
mod widgets;

#[derive(Resource)]
struct EmulatorFrame {
    frame_handle: Handle<Image>,
    frame_sprite: Entity,
}

#[derive(Resource)]
struct Machine {
    machine: DynamicMachine,
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

#[derive(Resource)]
struct Rom(Vec<u8>);

#[derive(Component)]
struct PickRom(Task<Option<(String, Vec<u8>)>>);

#[derive(Resource)]
struct UiData {
    paused: bool,
    frame_rate: f64,
    use_default_framerate: bool,
    cycles_per_frame: u32,
    actual_fps: f64,
    machine_model: DynamicModel,
    rom_name: Option<String>,
    palette: Palette,
}

impl Default for UiData {
    fn default() -> Self {
        Self {
            paused: false,
            frame_rate: model::CosmacVip.default_framerate(),
            use_default_framerate: true,
            cycles_per_frame: 1000,
            actual_fps: 0.0,
            machine_model: DynamicModel::CosmacVip,
            rom_name: None,
            palette: Default::default(),
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

const EMULATOR_TICK_RATE: DiagnosticPath = DiagnosticPath::const_new("emulator_tick_rate");

fn main() {
    println!("Hello, world!");

    App::new()
        .insert_resource(WinitSettings::game())
        .insert_resource(Time::<Fixed>::from_hz(model::CosmacVip.default_framerate()))
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
        .register_diagnostic(Diagnostic::new(EMULATOR_TICK_RATE))
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
        .add_audio_source::<Chip8Audio>()
        .run();
}

fn setup_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut beeper_assets: ResMut<Assets<Chip8Audio>>,
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
    commands.insert_resource(EmulatorFrame {
        frame_handle: handle,
        frame_sprite: sprite,
    });
    let beeper = Chip8Audio::new();
    let beeper_handle = beeper_assets.add(beeper.clone());
    commands.spawn(AudioPlayer(beeper_handle));
    commands.insert_resource(beeper);
}

fn update_ui_data(mut ui_data: ResMut<UiData>, diagnostics: Res<DiagnosticsStore>) {
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
                ui.label(ui_data.rom_name.as_deref().unwrap_or("No ROM loaded"));
                if ui.button("Choose ROM").clicked() {
                    ui_events.send(UiEvent::PickRom);
                }
            });

            model_selector(ui, &mut ui_data.machine_model);

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
                ui_data.use_default_framerate = false;
                ui_events.send(UiEvent::ChangeTickRate(ui_data.frame_rate));
            }
            if ui
                .toggle_value(
                    &mut ui_data.use_default_framerate,
                    "Use default FPS for system",
                )
                .changed()
                && ui_data.use_default_framerate
            {
                ui_data.frame_rate = ui_data.machine_model.default_framerate();
                ui_events.send(UiEvent::ChangeTickRate(ui_data.frame_rate));
            };

            ui.add(
                egui::Slider::new(&mut ui_data.cycles_per_frame, 1..=1000000)
                    .logarithmic(true)
                    .text("Cycles per frame"),
            );
        });

        palette_editor(ui, &mut ui_data.palette);

        ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover())
    });

    let available_rect = egui_to_bevy_rect(contexts.ctx_mut().available_rect());
    if available_rect != *last_available_rect {
        *last_available_rect = available_rect;
        ui_events.send(UiEvent::ResizeFrame(available_rect));
    }
}

fn handle_ui_events(
    mut commands: Commands,
    mut ui_data: ResMut<UiData>,
    mut ui_events: EventReader<UiEvent>,
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
                    if ui_data.use_default_framerate {
                        let rate = ui_data.machine_model.default_framerate();
                        ui_data.frame_rate = rate;
                        time.set_timestep_hz(rate);
                    }
                    commands.insert_resource(Machine::new(&ui_data.machine_model, &rom.0))
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
            .add_filter("Chip-8 ROMs", &["ch8", "xo8"])
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
        if let Some(maybe_rom) = block_on(poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            if let Some(rom) = maybe_rom {
                ui_data.rom_name = Some(rom.0);
                commands.insert_resource(Rom(rom.1));
            }
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
    write_frame(image, machine.machine.render_frame(&ui_data.palette));

    for _ in 0..(ui_data.cycles_per_frame) {
        if let Some((key, event)) = machine.queued_inputs.pop_front() {
            machine.machine.event(key, event);
        }
        if let Err(error) = machine.machine.tick() {
            error!("Emulator error: {error}");
            write_frame(image, machine.machine.render_frame(&ui_data.palette));
            commands.remove_resource::<Machine>();
            return;
        }
    }
}

fn machine_audio(
    machine: Option<Res<Machine>>,
    ui_data: Res<UiData>,
    mut audio: ResMut<Chip8Audio>,
) {
    // audio.edit(|audio| {
    //     if let Some(machine) = machine.as_ref() {
    //         audio.set_active(machine.machine.sound_active() && !ui_data.paused);
    //         audio.set_pitch(machine.machine.pitch());
    //         audio.set_pattern(*machine.machine.audio_pattern());
    //     } else {
    //         *audio = Default::default();
    //     }
    // });
    if let Some(machine) = machine
            .as_ref()
            .filter(|machine| machine.machine.sound_active() && !ui_data.paused)
        {
            audio.render_audio(
                machine.machine.pitch(),
                *machine.machine.audio_pattern(),
                1.0 / ui_data.frame_rate,
            );
        }
}

fn write_frame(texture: &mut Image, frame: RgbaImage) {
    if texture.width() != frame.width() || texture.height() != texture.height() {
        texture.resize(Extent3d {
            width: frame.width(),
            height: frame.height(),
            depth_or_array_layers: 1,
        });
    }
    texture.data = frame.into_vec();
}

fn egui_to_bevy_rect(rect: egui::Rect) -> Rect {
    Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y)
}
