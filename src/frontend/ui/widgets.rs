use std::fmt::Display;

use bevy_egui::egui::{self, reset_button_with, Align, Color32, Response, Ui};
use image::Rgba;

use crate::{
    hardware::KeyEvent,
    model::{DrawWaitSetting, DynamicModel, Quirks},
    screen::Palette,
};

pub fn model_selector(ui: &mut Ui, model: &mut DynamicModel) -> egui::Response {
    egui::ComboBox::from_label("Machine type")
        .selected_text(model.to_string())
        .show_ui(ui, |ui| {
            ui.selectable_value(
                model,
                DynamicModel::COSMAC_VIP,
                DynamicModel::COSMAC_VIP.to_string(),
            );
            ui.selectable_value(
                model,
                DynamicModel::LEGACY_SCHIP,
                DynamicModel::LEGACY_SCHIP.to_string(),
            );
            ui.selectable_value(
                model,
                DynamicModel::MODERN_SCHIP,
                DynamicModel::MODERN_SCHIP.to_string(),
            );
            ui.selectable_value(
                model,
                DynamicModel::XO_CHIP,
                DynamicModel::XO_CHIP.to_string(),
            );
        })
        .response
}

pub fn palette_editor(ui: &mut Ui, palette: &mut Palette) -> egui::CollapsingResponse<()> {
    egui::CollapsingHeader::new("Customize Palette").show(ui, |ui| {
        ui.checkbox(
            &mut palette.use_custom_two_color,
            "Use custom colors for two-color mode",
        );
        ui.add_enabled_ui(palette.use_custom_two_color, |ui| {
            ui.horizontal(|ui| {
                ui.label("Off:");
                color_edit_button(ui, &mut palette.two_color[0]);
                ui.label("On:");
                color_edit_button(ui, &mut palette.two_color[1]);
            });
        });
        ui.separator();
        egui::Grid::new("palette-grid").show(ui, |ui| {
            for (i, color) in palette.sixteen_color.iter_mut().enumerate() {
                ui.with_layout(ui.layout().with_main_align(Align::Max), |ui| {
                    color_edit_button(ui, color);
                });
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
        });
    })
}

pub fn color_edit_button(ui: &mut Ui, color: &mut Rgba<u8>) -> Response {
    let mut egui_color =
        Color32::from_rgba_premultiplied(color.0[0], color.0[1], color.0[2], color.0[3]);
    let response = ui.color_edit_button_srgba(&mut egui_color);
    if response.changed() {
        *color = Rgba::from(egui_color.to_array());
    }
    response
}

pub fn edit_quirks(
    ui: &mut Ui,
    quirks: &mut Quirks,
    default: Quirks,
) -> egui::CollapsingResponse<()> {
    egui::CollapsingHeader::new("Edit Quirks").show(ui, |ui| {
        if ui.button("Reset all to default").clicked() {
            *quirks = default;
        }

        let boolean_options = [
            (
                &mut quirks.graceful_exit_on_0000,
                default.graceful_exit_on_0000,
                "Exit gracefully on opcode 0x0000.",
            ),
            (
                &mut quirks.bitshift_use_y,
                default.bitshift_use_y,
                "Bitshift operations read from vY instead of vX.",
            ),
            (
                &mut quirks.inc_i_on_slice,
                default.inc_i_on_slice,
                "Increment I by X+1 on bulk load/store.",
            ),
            (
                &mut quirks.bitwise_reset_flag,
                default.bitwise_reset_flag,
                "Bitwise instructions (8xy1 OR/8xy2 AND/8xy3 XOR) reset vF.",
            ),
            (
                &mut quirks.clear_screen_on_mode_switch,
                default.clear_screen_on_mode_switch,
                "Clear the screen when switching between hires and lores modes.",
            ),
            (
                &mut quirks.jump_v0_use_vx,
                default.jump_v0_use_vx,
                "Bnnn (jump to NNN+v0) instead becomes Bxnn (jump to NNN+vX).",
            ),
            (
                &mut quirks.lores_draw_large_as_small,
                default.lores_draw_large_as_small,
                "In lores mode, Dxy0 (draw 16x16 sprite) instead draws a small sprite with height 16.",
            ),
        ];

        for (value, default_value, text) in boolean_options {
            draw_quirk_config_option(ui, value, default_value, text, Ui::checkbox);
        }
        draw_quirk_config_option(
            ui,
            &mut quirks.key_wait_trigger,
            default.key_wait_trigger,
            "Which key event triggers Fx0A (wait for key)?",
            |ui, value, text| {
                egui::ComboBox::from_id_salt("quirks_key_wait_trigger")
                    .selected_text(value.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(value, KeyEvent::Press, KeyEvent::Press.to_string());
                        ui.selectable_value(
                            value,
                            KeyEvent::Release,
                            KeyEvent::Release.to_string(),
                        );
                    });
                ui.label(text)
            },
        );
        draw_quirk_config_option(
            ui,
            &mut quirks.draw_wait_for_vblank,
            default.draw_wait_for_vblank,
            "Dxyn and Dxy0 (draw sprite) wait for vblank (end of the frame) to draw.",
            |ui, value, text| {
                egui::ComboBox::from_id_salt("quirks_draw_wait_for_vblank")
                    .selected_text(value.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            value,
                            DrawWaitSetting::Always,
                            DrawWaitSetting::Always.to_string(),
                        );
                        ui.selectable_value(
                            value,
                            DrawWaitSetting::LoresOnly,
                            DrawWaitSetting::LoresOnly.to_string(),
                        );
                        ui.selectable_value(
                            value,
                            DrawWaitSetting::Never,
                            DrawWaitSetting::Never.to_string(),
                        );
                    });
                ui.label(text)
            },
        );
    })
}

fn draw_quirk_config_option<T: Display + PartialEq>(
    ui: &mut Ui,
    value: &mut T,
    default_value: T,
    text: &str,
    render: impl FnOnce(&mut Ui, &mut T, String) -> egui::Response,
) -> egui::Response {
    let label = format!("{} Default: {}", text, default_value);
    let id = egui::Id::new(text);
    let height = ui
        .memory(|memory| memory.data.get_temp::<f32>(id))
        .unwrap_or(ui.available_size_before_wrap().y);
    let response = ui.allocate_ui_with_layout(
        egui::vec2(ui.available_size_before_wrap().x, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            reset_button_with(ui, value, "⟲", default_value);
            render(ui, value, label)
        },
    );
    ui.memory_mut(|memory| {
        memory
            .data
            .insert_temp(egui::Id::new(text), response.inner.rect.height())
    });
    response.response
}
