use bevy::{
    color::palettes::css,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    tasks::{block_on, poll_once, IoTaskPool, Task},
    winit::WinitSettings,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use hardware::{Chip8, KeyEvent};

mod hardware;
mod instruction;
mod model;
mod screen;

#[derive(Resource)]
struct EmulatorFrame(Handle<Image>, Entity);

#[derive(Resource)]
struct Machine(Chip8<model::CosmacVip>);

#[derive(Resource)]
struct Rom(String, Vec<u8>);

#[derive(Component)]
struct PickRom(Task<Option<Rom>>);

#[derive(Resource, Default)]
struct UiData {
    paused: bool,
    tick_rate: f64,
    fps: f64,
    rom_name: String,
}

#[derive(Event)]
enum UiEvent {
    PickRom,
    ResetMachine,
    SetPause(bool),
    ChangeTickRate(f64),
    ResizeFrame(Rect),
}

fn main() {
    println!("Hello, world!");

    App::new()
        .insert_resource(WinitSettings::game())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<UiData>()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_event::<UiEvent>()
        .add_systems(Startup, setup_system)
        .add_systems(PreUpdate, update_ui_data)
        .add_systems(Update, (emulator_ui_system, rom_loaded))
        .add_systems(PostUpdate, handle_ui_events)
        .add_systems(
            FixedUpdate,
            machine_system.run_if(resource_exists::<Machine>),
        )
        .run();
}

fn setup_system(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
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
    commands.insert_resource(EmulatorFrame(handle, sprite));
}

fn update_ui_data(
    mut ui_data: ResMut<UiData>,
    time: (Res<Time<Virtual>>, Res<Time<Fixed>>),
    rom: Option<Res<Rom>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    ui_data.paused = time.0.is_paused();
    ui_data.tick_rate = 1.0 / time.1.timestep().as_secs_f64();
    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    {
        ui_data.fps = fps;
    }
    ui_data.rom_name = rom
        .as_ref()
        .map(|rom| rom.0.as_str())
        .unwrap_or("No ROM loaded")
        .to_owned();
}

fn emulator_ui_system(
    mut contexts: EguiContexts,
    mut ui_data: ResMut<UiData>,
    mut ui_events: EventWriter<UiEvent>,
    mut last_available_rect: Local<Rect>,
) {
    egui::SidePanel::new(egui::panel::Side::Left, "control-panel").show(contexts.ctx_mut(), |ui| {
        ui.label(format!("FPS: {}", ui_data.fps));

        egui::Frame::none().outer_margin(4.0).show(ui, |ui| {
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

        ui.horizontal(|ui| {
            if ui.toggle_value(&mut ui_data.paused, "Pause").changed() {
                ui_events.send(UiEvent::SetPause(ui_data.paused));
            }
            let original_tick_rate = ui_data.tick_rate;
            ui.add(egui::Slider::new(&mut ui_data.tick_rate, 0.5..=120.0).step_by(0.5));
            if ui_data.tick_rate != original_tick_rate {
                ui_events.send(UiEvent::ChangeTickRate(ui_data.tick_rate));
            }
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
    mut time: (ResMut<Time<Virtual>>, ResMut<Time<Fixed>>),
    rom: Option<Res<Rom>>,
    frame_handle: Res<EmulatorFrame>,
    mut frame_sprite_query: Query<(&mut Sprite, &mut Transform)>,
) {
    for event in ui_events.read() {
        match event {
            UiEvent::PickRom => {
                commands.spawn(pick_rom());
            }
            UiEvent::ResetMachine => {
                if let Some(rom) = rom.as_ref() {
                    commands.insert_resource(Machine(Chip8::new(model::CosmacVip, &rom.1)))
                }
            }
            UiEvent::SetPause(true) => time.0.pause(),
            UiEvent::SetPause(false) => time.0.unpause(),
            UiEvent::ChangeTickRate(rate) => time.1.set_timestep_hz(*rate),
            UiEvent::ResizeFrame(rect) => {
                let (mut sprite, mut transform) = frame_sprite_query
                    .get_mut(frame_handle.1)
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
            Ok(data) => Some(Rom(
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

fn rom_loaded(mut commands: Commands, mut tasks: Query<(Entity, &mut PickRom)>) {
    for (entity, mut task) in &mut tasks {
        if let Some(Some(rom)) = block_on(poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            commands.insert_resource(rom);
        }
    }
}

const CYCLES_PER_TICK: usize = 1;

fn machine_system(
    mut commands: Commands,
    mut machine: ResMut<Machine>,
    frame_handle: Res<EmulatorFrame>,
    mut images: ResMut<Assets<Image>>,
) {
    for _ in 0..CYCLES_PER_TICK {
        if let Err(error) = machine.0.tick() {
            error!("Emulator error: {error}");
            commands.remove_resource::<Machine>();
            return;
        }
    }

    let image = images
        .get_mut(&frame_handle.0)
        .expect("Emulator frame not found");
    image.data = machine.0.render_frame().into_vec();

    machine.0.tick_timers();
}

fn egui_to_bevy_rect(rect: egui::Rect) -> Rect {
    Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y)
}
