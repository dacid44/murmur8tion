use bevy_egui::egui::{self, Align, Color32, InnerResponse, Response, Ui, WidgetText};
use image::Rgba;

use crate::screen::Palette;

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
