use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use widgets::{model_selector, palette_editor};

use crate::model::Model;

use super::{DisplayRect, EmulatorData, EmulatorEvent};

mod widgets;

pub fn ui_plugin(app: &mut App) {
    app.add_plugins(EguiPlugin)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_systems(Update, emulator_ui_system);
}

fn emulator_ui_system(
    mut contexts: EguiContexts,
    diagnostics: Res<DiagnosticsStore>,
    mut ui_data: ResMut<EmulatorData>,
    mut ui_events: EventWriter<EmulatorEvent>,
    mut display_rect: ResMut<DisplayRect>,
) {
    let egui_context = contexts.ctx_mut();
    egui::SidePanel::new(egui::panel::Side::Left, "control-panel").show(egui_context, |ui| {
        ui.label(format!(
            "FPS: {}",
            diagnostics
                .get(&FrameTimeDiagnosticsPlugin::FPS)
                .and_then(|fps| fps.smoothed())
                .unwrap_or(0.0)
        ));

        egui::Frame::group(&egui_context.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(ui_data.rom_name.as_deref().unwrap_or("No ROM loaded"));
                if ui.button("Choose ROM").clicked() {
                    ui_events.send(EmulatorEvent::PickRom);
                }
            });

            model_selector(ui, &mut ui_data.machine_model);

            if ui.button("Reset Emulator").clicked() {
                ui_events.send(EmulatorEvent::ResetMachine);
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
                ui_events.send(EmulatorEvent::ChangeTickRate(ui_data.frame_rate));
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
                ui_events.send(EmulatorEvent::ChangeTickRate(ui_data.frame_rate));
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
    if available_rect != display_rect.0 {
        display_rect.0 = available_rect;
    }
}

fn egui_to_bevy_rect(rect: egui::Rect) -> Rect {
    Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y)
}
