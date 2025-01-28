use std::{
    collections::VecDeque,
    sync::mpsc,
    thread::spawn,
    time::{Duration, Instant},
};

use egui_macroquad::{egui, macroquad};
use hardware::{Chip8, KeyEvent};
use macroquad::prelude::*;
use ux::u4;

mod hardware;
mod instruction;
mod model;
mod screen;

const ROM: &[u8] = include_bytes!("../roms/Timendus/2-ibm-logo.ch8");

#[macroquad::main("Chip8 Emulator")]
async fn main() {
    println!("Hello, world!");

    let mut screen_texture = Texture2D::empty();
    let mut screen_texture_size = (1, 1);
    let (tx, machine_rx) = mpsc::channel();
    let (machine_tx, rx) = mpsc::channel();
    let emulator_thread = std::thread::spawn(move || run_machine(machine_tx, machine_rx));
    tx.send(ToMachine::Reset(
        model::CosmacVip,
        ROM.to_owned().into_boxed_slice(),
    ))
    .expect("Emulator thread exited");

    loop {
        for message in rx.try_iter() {
            match message {
                FromMachine::Frame(frame, _) => {
                    let frame_size = (frame.width(), frame.height());
                    if frame_size == screen_texture_size {
                        screen_texture.update(&frame);
                    } else {
                        screen_texture = Texture2D::from_image(&frame);
                        screen_texture_size = frame_size;
                    }
                }
                FromMachine::Error(error) => eprintln!("Emulator error: {error}"),
            }
        }

        clear_background(GREEN);

        // Process keys

        let mut remaining = egui::Rect::NOTHING;
        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::new(egui::panel::Side::Left, "control-panel").show(egui_ctx, |ui| {
                ui.label("Test");
                ui.button("Test");
            });
            remaining = egui_ctx.available_rect();
        });

        let texture_size = egui::vec2(screen_texture.width(), screen_texture.height());
        let scale = (remaining.size() / texture_size).min_elem();
        let texture_rect = egui::Rect::from_center_size(remaining.center(), texture_size * scale);

        egui_macroquad::draw();

        draw_texture_ex(
            screen_texture,
            texture_rect.left(),
            texture_rect.top(),
            RED,
            DrawTextureParams {
                dest_size: Some(macroquad::math::Vec2::from_array(
                    texture_rect.size().into(),
                )),
                ..Default::default()
            },
        );

        next_frame().await;
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
    Frame(Image, Duration),
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
