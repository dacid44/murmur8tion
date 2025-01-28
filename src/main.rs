use std::{
    collections::VecDeque,
    sync::{mpsc, Arc},
    thread::spawn,
    time::{Duration, Instant},
};

use bevy::{
    color::palettes::css,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    utils::synccell::SyncCell,
    window::PrimaryWindow,
    winit::WinitSettings,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use hardware::{Chip8, KeyEvent};
use image::RgbaImage;
use ux::u4;

mod hardware;
mod instruction;
mod model;
mod screen;

const ROM: &[u8] = include_bytes!("../roms/Timendus/2-ibm-logo.ch8");

#[derive(Default, Resource)]
struct PanelSize(f32);

#[derive(Resource)]
struct EmulatorFrame(Handle<Image>, Entity);

#[derive(Resource)]
struct MachineThread {
    handle: std::thread::JoinHandle<()>,
    tx: mpsc::Sender<ToMachine>,
    rx: SyncCell<mpsc::Receiver<FromMachine>>,
}

fn main() {
    println!("Hello, world!");

    App::new()
        .insert_resource(WinitSettings::game())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .init_resource::<PanelSize>()
        .add_systems(Startup, setup_system)
        .add_systems(PreUpdate, message_handler_system)
        .add_systems(Update, emulator_ui_system)
        .run();

    // Process keys

    // let mut remaining = egui::Rect::NOTHING;
    // egui_macroquad::ui(|egui_ctx| {
    //     egui::SidePanel::new(egui::panel::Side::Left, "control-panel").show(egui_ctx, |ui| {
    //         ui.label("Test");
    //         ui.button("Test");
    //     });
    //     remaining = egui_ctx.available_rect();
    // });

    // let texture_size = egui::vec2(screen_texture.width(), screen_texture.height());
    // let scale = (remaining.size() / texture_size).min_elem();
    // let texture_rect = egui::Rect::from_center_size(remaining.center(), texture_size * scale);
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

    let (tx, machine_rx) = mpsc::channel();
    let (machine_tx, rx) = mpsc::channel();
    let machine_thread = std::thread::spawn(move || run_machine(machine_tx, machine_rx));
    tx.send(ToMachine::Reset(
        model::CosmacVip,
        ROM.to_owned().into_boxed_slice(),
    ))
    .expect("Emulator thread exited");
    commands.insert_resource(MachineThread {
        handle: machine_thread,
        tx,
        rx: SyncCell::new(rx),
    });
}

fn message_handler_system(
    mut machine_thread: ResMut<MachineThread>,
    frame_handle: Res<EmulatorFrame>,
    mut images: ResMut<Assets<Image>>,
) {
    for message in machine_thread.rx.get().try_iter() {
        match message {
            FromMachine::Frame(frame, _) => {
                let image = images
                    .get_mut(&frame_handle.0)
                    .expect("Emulator frame not found");
                image.data = frame.into_vec();
            }
            FromMachine::Error(error) => eprintln!("Emulator error: {error}"),
        }
    }
}

fn emulator_ui_system(
    mut contexts: EguiContexts,
    mut panel_size: Local<f32>,
    frame_handle: Res<EmulatorFrame>,
    mut frame_sprite_query: Query<(&mut Sprite, &mut Transform)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    let new_panel_size = egui::SidePanel::new(egui::panel::Side::Left, "control-panel")
        .show(contexts.ctx_mut(), |ui| {
            if let Some(value) = diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|fps| fps.smoothed())
            {
                ui.label(format!("FPS: {value}"));
            }
            ui.label("Test");
            ui.button("Test");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover())
        })
        .response
        .rect
        .width();

    if new_panel_size != *panel_size {
        *panel_size = new_panel_size;

        let window = windows.single();
        let (mut sprite, mut transform) = frame_sprite_query
            .get_mut(frame_handle.1)
            .expect("Could not find frame entity");

        let frame_size = Vec2::new(
            screen::CosmacVipScreen::WIDTH as f32,
            screen::CosmacVipScreen::HEIGHT as f32,
        );
        let mut window_size = window.size();
        window_size.x -= new_panel_size;
        let scale = (window_size / frame_size).min_element();
        sprite.custom_size = Some(frame_size * scale);

        transform.translation.x = new_panel_size / 2.0;
    }
}

#[derive(Debug)]
enum ToMachine {
    Reset(model::CosmacVip, Box<[u8]>),
    Exit,
    Input(u4, KeyEvent),
}

#[derive(Debug)]
enum FromMachine {
    Frame(RgbaImage, Duration),
    Error(hardware::Error),
}

const FRAME_RATE: f32 = 4.0;
const CYCLES_PER_FRAME: usize = 1;

fn run_machine(tx: mpsc::Sender<FromMachine>, rx: mpsc::Receiver<ToMachine>) {
    let mut machine_option = None;

    let mut next_frame = Instant::now();
    let mut last_frame = Instant::now();
    let mut events = VecDeque::new();
    'outer: loop {
        for message in rx.try_iter() {
            match message {
                ToMachine::Reset(model, rom) => machine_option = Some(Chip8::new(model, &rom)),
                ToMachine::Exit => break 'outer,
                ToMachine::Input(key, event) => events.push_back((key, event)),
            }
        }

        if let Some(machine) = machine_option.as_mut() {
            for _ in 0..CYCLES_PER_FRAME {
                if let Some((key, event)) = events.pop_front() {
                    machine.event(key, event);
                }
                if let Err(error) = machine.cycle() {
                    tx.send(FromMachine::Error(error))
                        .expect("Main thread exited");
                    machine_option = None;
                    break;
                }
            }
        }

        next_frame += Duration::from_secs_f32(1.0 / FRAME_RATE);
        std::thread::sleep(next_frame - Instant::now());

        if let Some(machine) = machine_option.as_mut() {
            let frame = machine.render_frame();
            machine.frame();
            let ts = Instant::now();
            tx.send(FromMachine::Frame(frame, ts - last_frame))
                .expect("Main thread exited");
            last_frame = ts;
        } else {
            events.clear();
            last_frame = Instant::now();
        }
    }
}
