use std::sync::Arc;

use bevy::{ecs::system::ResMut, render::camera::ClearColor};
use bevy_egui::{
    egui::{self, style::HandleShape, Color32},
    EguiContexts,
};

pub const NIGHT: Color32 = Color32::from_rgb(7, 11, 11); // #070B0B
pub const SPACE_CADET: Color32 = Color32::from_rgb(39, 36, 59); // #27243B
pub const SPACE_CADET_FAINT: Color32 = Color32::from_rgb(46, 43, 68); // #2E2B44
pub const SPACE_CADET_DARK: Color32 = Color32::from_rgb(32, 29, 51); // #201D33
pub const RAISIN_BLACK: Color32 = Color32::from_rgb(44, 42, 55); // #2CA37
pub const JASPER: Color32 = Color32::from_rgb(203, 79, 53); // #CB4F35
pub const AUBURN: Color32 = Color32::from_rgb(172, 61, 56); // #AC3D38
pub const BURNT_UMBER: Color32 = Color32::from_rgb(129, 48, 44); // # 81302C
pub const FIRE_BRICK: Color32 = Color32::from_rgb(177, 67, 46); // #B1432E
pub const OLD_ROSE: Color32 = Color32::from_rgb(190, 143, 143); // #BE8F8F
pub const QUINACRIDONE_MAGENTA: Color32 = Color32::from_rgb(125, 62, 80); // #7D3E50
pub const WINE: Color32 = Color32::from_rgb(97, 47, 54); // #612F36
pub const CAPUT_MORTUUM: Color32 = Color32::from_rgb(79, 39, 45); // #4F272D
pub const HUNYADI_YELLOW_ORIGINAL: Color32 = Color32::from_rgb(227, 164, 60); // #E3A43C
pub const HUNYADI_YELLOW: Color32 = Color32::from_rgb(250, 181, 70); // #FAB546
pub const JET: Color32 = Color32::from_rgb(47, 47, 50); // #2F2F32
pub const WENGE: Color32 = Color32::from_rgb(99, 90, 92); // #635A5C
pub const TAUPE_GRAY: Color32 = Color32::from_rgb(159, 152, 152); // #9F9898
pub const CINEROUS: Color32 = Color32::from_rgb(139, 120, 116); // #8B7874

pub const BACKGROUND: Color32 = SPACE_CADET_DARK;
pub const BACKGROUND_FAINT: Color32 = SPACE_CADET_FAINT;
pub const BACKGROUND_DARK: Color32 = NIGHT;
pub const BACKGROUND_NEUTRAL: Color32 = RAISIN_BLACK;
pub const FOREGROUND_LIGHT: Color32 = OLD_ROSE;
pub const FOREGROUND_MID: Color32 = QUINACRIDONE_MAGENTA;
pub const FOREGROUND_MID_DARK: Color32 = WINE;
pub const FOREGROUND_DARK: Color32 = CAPUT_MORTUUM;
pub const ACCENT_LIGHT: Color32 = HUNYADI_YELLOW;
pub const ACCENT_MID: Color32 = FIRE_BRICK;
pub const ACCENT_DARK: Color32 = BURNT_UMBER;
pub const NEUTRAL_LIGHT: Color32 = TAUPE_GRAY;
pub const NEUTRAL_MID: Color32 = WENGE;
pub const NEUTRAL_DARK: Color32 = JET;
pub const NEUTRAL_ACCENT: Color32 = CINEROUS;

const PIXEL_CODE_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/PixelCode/patched/PixelCode.otf"
));

const OUTLINE_STROKE_WIDTH: f32 = 2.0;

pub const LARGE_BUTTON_SIZE: egui::Vec2 = egui::vec2(36.0, 36.0);

pub fn apply_style(mut contexts: EguiContexts, mut clear_color: ResMut<ClearColor>) {
    contexts.ctx_mut().all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        let widgets = &mut visuals.widgets;
        let spacing = &mut style.spacing;

        visuals.handle_shape = HandleShape::Rect { aspect_ratio: 1.0 };
        visuals.menu_rounding = egui::Rounding::ZERO;
        widgets.noninteractive.rounding = egui::Rounding::ZERO;
        widgets.inactive.rounding = egui::Rounding::ZERO;
        widgets.hovered.rounding = egui::Rounding::ZERO;
        widgets.active.rounding = egui::Rounding::ZERO;
        widgets.open.rounding = egui::Rounding::ZERO;

        visuals.slider_trailing_fill = true;
        visuals.text_cursor.stroke.color = FOREGROUND_LIGHT;

        visuals.dark_mode = true;
        visuals.window_fill = BACKGROUND;
        visuals.window_stroke.color = FOREGROUND_DARK;
        visuals.panel_fill = BACKGROUND;
        visuals.faint_bg_color = BACKGROUND_FAINT;
        visuals.extreme_bg_color = BACKGROUND_DARK;
        visuals.code_bg_color = BACKGROUND_DARK;
        visuals.warn_fg_color = ACCENT_MID;
        visuals.error_fg_color = ACCENT_DARK;

        visuals.selection.bg_fill = ACCENT_DARK;
        visuals.selection.stroke.color = FOREGROUND_LIGHT;

        widgets.noninteractive.bg_fill = BACKGROUND;
        widgets.noninteractive.weak_bg_fill = BACKGROUND;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, FOREGROUND_DARK);
        widgets.noninteractive.fg_stroke =
            egui::Stroke::new(OUTLINE_STROKE_WIDTH, FOREGROUND_LIGHT);

        widgets.inactive.bg_fill = FOREGROUND_DARK;
        widgets.inactive.weak_bg_fill = FOREGROUND_DARK;
        widgets.inactive.bg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, ACCENT_LIGHT);
        widgets.inactive.fg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, FOREGROUND_LIGHT);

        widgets.hovered.bg_fill = FOREGROUND_MID;
        widgets.hovered.weak_bg_fill = FOREGROUND_MID;
        widgets.hovered.bg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, ACCENT_LIGHT);
        widgets.hovered.fg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, FOREGROUND_LIGHT);
        widgets.hovered.expansion = 2.0;

        widgets.active.bg_fill = FOREGROUND_MID_DARK;
        widgets.active.weak_bg_fill = FOREGROUND_MID_DARK;
        widgets.active.bg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, ACCENT_LIGHT);
        widgets.active.fg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, FOREGROUND_LIGHT);
        widgets.active.expansion = 2.0;

        widgets.open.bg_fill = FOREGROUND_DARK;
        widgets.open.weak_bg_fill = FOREGROUND_DARK;
        widgets.open.bg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, ACCENT_LIGHT);
        widgets.open.fg_stroke = egui::Stroke::new(OUTLINE_STROKE_WIDTH, FOREGROUND_LIGHT);
        widgets.open.expansion = 2.0;

        spacing.item_spacing = egui::vec2(8.0, 8.0);
        spacing.icon_spacing = 8.0;
        spacing.indent = 24.0;
    });

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Pixel Code".to_owned(),
        Arc::new(
            egui::FontData::from_static(PIXEL_CODE_FONT).tweak(egui::FontTweak {
                scale: 1.2,
                ..Default::default()
            }),
        ),
    );
    fonts.font_data.insert(
        "Pixel Code SlightlyRaised".to_owned(),
        Arc::new(
            egui::FontData::from_static(PIXEL_CODE_FONT).tweak(egui::FontTweak {
                scale: 1.2,
                y_offset_factor: -0.05,
                ..Default::default()
            }),
        ),
    );
    fonts.font_data.insert(
        "Pixel Code Raised".to_owned(),
        Arc::new(
            egui::FontData::from_static(PIXEL_CODE_FONT).tweak(egui::FontTweak {
                scale: 1.2,
                y_offset_factor: -0.143,
                ..Default::default()
            }),
        ),
    );
    let fallback_order = vec![
        "Pixel Code".to_owned(),
        "NotoEmoji-Regular".to_owned(),
        "emoji-icon-font".to_owned(),
    ];
    fonts
        .families
        .insert(egui::FontFamily::Proportional, fallback_order.clone());
    fonts
        .families
        .insert(egui::FontFamily::Monospace, fallback_order);
    fonts
        .families
        .insert(egui::FontFamily::Name("Pixel Code Raised".into()), vec!["Pixel Code Raised".to_owned()]);
    fonts
        .families
        .insert(egui::FontFamily::Name("Pixel Code SlightlyRaised".into()), vec!["Pixel Code SlightlyRaised".to_owned()]);
    contexts.ctx_mut().set_fonts(fonts);

    clear_color.0 = egui_to_bevy_color(BACKGROUND_DARK);
}

pub fn egui_to_bevy_color(color: Color32) -> bevy::color::Color {
    let [r, g, b, a] = color.to_array();
    bevy::color::Color::Srgba(bevy::color::Srgba::rgba_u8(r, g, b, a))
}
