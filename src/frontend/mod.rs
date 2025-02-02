use audio::Chip8Audio;
use bevy::{
    asset::RenderAssetUsages,
    audio::AddAudioSource,
    color::palettes::css,
    diagnostic::{Diagnostic, DiagnosticPath, RegisterDiagnostic},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use machine::machine_plugin;

use crate::{
    model::{self, DynamicModel, Model},
    screen::Palette,
};

pub mod audio;
mod machine;
mod rom;
mod ui;

#[derive(Resource)]
struct FrameHandle(Handle<Image>);

#[derive(Resource, Default)]
struct DisplayRect(Rect);

#[derive(Component)]
#[require(Transform)]
struct ScaleToDisplay(Vec2);

#[derive(Resource)]
struct EmulatorData {
    paused: bool,
    frame_rate: f64,
    use_default_framerate: bool,
    cycles_per_frame: u32,
    machine_model: DynamicModel,
    rom_name: Option<String>,
    palette: Palette,
}

impl Default for EmulatorData {
    fn default() -> Self {
        Self {
            paused: false,
            frame_rate: model::CosmacVip.default_framerate(),
            use_default_framerate: true,
            cycles_per_frame: 1000,
            machine_model: DynamicModel::CosmacVip,
            rom_name: None,
            palette: Default::default(),
        }
    }
}

#[derive(Event)]
enum EmulatorEvent {
    PickRom,
    ResetMachine,
    ChangeTickRate(f64),
}

const EMULATOR_TICK_RATE: DiagnosticPath = DiagnosticPath::const_new("emulator_tick_rate");

const FRAME_ASPECT_RATIO: Vec2 = Vec2::new(2.0, 1.0);

pub fn emulator_plugin(app: &mut App) {
    app.init_resource::<EmulatorData>()
        .init_resource::<DisplayRect>()
        .add_event::<EmulatorEvent>()
        .add_audio_source::<Chip8Audio>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            scale_display.run_if(resource_changed::<DisplayRect>),
        )
        .register_diagnostic(Diagnostic::new(EMULATOR_TICK_RATE))
        .add_plugins((machine::machine_plugin, ui::ui_plugin, rom::rom_plugin));
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut audio_assets: ResMut<Assets<Chip8Audio>>,
) {
    commands.spawn(Camera2d);

    let image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &(css::BEIGE.to_u8_array()),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(image);

    let mut sprite = Sprite::from_image(handle.clone());
    sprite.custom_size = Some(FRAME_ASPECT_RATIO);
    commands.spawn((sprite, ScaleToDisplay(FRAME_ASPECT_RATIO)));
    commands.insert_resource(FrameHandle(handle));

    let audio = Chip8Audio::new();
    let beeper_handle = audio_assets.add(audio.clone());
    commands.spawn(AudioPlayer(beeper_handle));
    commands.insert_resource(audio);
}

fn scale_display(
    display_rect: Res<DisplayRect>,
    mut display_transforms: Query<(&mut Transform, &ScaleToDisplay)>,
) {
    for (mut transform, ratio) in display_transforms.iter_mut() {
        let scale = (display_rect.0.size() / ratio.0).min_element();
        transform.translation.x = display_rect.0.min.x / 2.0;
        transform.scale = Vec3::new(scale, scale, 1.0);
    }
}
