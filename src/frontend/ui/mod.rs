use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::egui::{self, Ui};
use widgets::{edit_quirks, model_selector, palette_editor};

use crate::model::{self, Model};

use super::{
    debug::{show_debug_options, DebugOptions},
    EmulatorData, EmulatorEvent,
};

pub mod style;
mod widgets;

pub fn ui_plugin(app: &mut App) {
    app.add_plugins(FrameTimeDiagnosticsPlugin)
        .add_systems(Startup, style::apply_style);
}

pub fn draw_main_ui(
    ui: InMut<Ui>,
    diagnostics: Res<DiagnosticsStore>,
    mut emulator_data: ResMut<EmulatorData>,
    mut events: EventWriter<EmulatorEvent>,
    mut debug_options: ResMut<DebugOptions>,
) {
    ui.0.label(format!(
        "FPS: {}",
        diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
            .unwrap_or(0.0)
    ));

    ui.0.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(emulator_data.rom_name.as_deref().unwrap_or("No ROM loaded"));
            if ui.button("Choose ROM").clicked() {
                events.send(EmulatorEvent::PickRom);
            }
        });

        model_selector(ui, &mut emulator_data.machine_model);

        if ui.button("Reset Emulator").clicked() {
            events.send(EmulatorEvent::ResetMachine);
        }
    });

    ui.0.group(|ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

            ui.toggle_value(&mut emulator_data.paused, "Pause");

            let original_tick_rate = emulator_data.frame_rate;
            ui.add(
                egui::Slider::new(&mut emulator_data.frame_rate, 0.5..=120.0)
                    .step_by(0.5)
                    .text("Target FPS"),
            );
            if emulator_data.frame_rate != original_tick_rate {
                emulator_data.use_default_framerate = false;
                events.send(EmulatorEvent::ChangeTickRate(emulator_data.frame_rate));
            }
            if ui
                .toggle_value(
                    &mut emulator_data.use_default_framerate,
                    "Use default FPS for system",
                )
                .changed()
                && emulator_data.use_default_framerate
            {
                emulator_data.frame_rate = emulator_data.machine_model.default_framerate();
                events.send(EmulatorEvent::ChangeTickRate(emulator_data.frame_rate));
            };

            ui.add(
                egui::Slider::new(&mut emulator_data.cycles_per_frame, 1..=1000000)
                    .logarithmic(true)
                    .text("Cycles per frame"),
            );

            palette_editor(ui, &mut emulator_data.palette);
            show_debug_options(ui, &mut debug_options);
            let default_quirks = emulator_data.machine_model.default_quirks();
            edit_quirks(ui, emulator_data.machine_model.quirks_mut(), default_quirks);

            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover())
        });
    });
}
