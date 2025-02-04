use std::fmt::Display;

use bevy::{color::palettes::css, prelude::*, render::view::VisibilitySystems};
use bevy_egui::{
    egui::{self, Ui},
    EguiContexts,
};
use bevy_inspector_egui::bevy_inspector;

use super::{layout::ScaleToDisplay, machine::Machine, ui::style, Frame, FRAME_ASPECT_RATIO};

#[derive(Resource, Clone, Default)]
pub struct DebugOptions {
    debug_grid: GridSize,
}

#[derive(Component)]
#[require(ScaleToDisplay(|| ScaleToDisplay(FRAME_ASPECT_RATIO)), Transform, InheritedVisibility)]
struct DebugGrid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum GridSize {
    #[default]
    None,
    Eight,
    Four,
    Two,
}

impl Display for GridSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridSize::None => write!(f, "None"),
            GridSize::Eight => write!(f, "8 Pixels"),
            GridSize::Four => write!(f, "4 Pixels"),
            GridSize::Two => write!(f, "2 Pixels"),
        }
    }
}

pub fn debug_plugin(app: &mut App) {
    app.init_resource::<DebugOptions>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // bevy_inspector_ui.run_if(options_func(|opts| opts.show_inspector)),
                render_grid
                    .after(VisibilitySystems::VisibilityPropagate)
                    .run_if(options_func(|opts| opts.debug_grid > GridSize::None)),
            ),
        );
}

fn options_func(f: impl Fn(&DebugOptions) -> bool + Send + Sync + 'static) -> impl Condition<()> {
    IntoSystem::into_system(move |debug_options: Res<DebugOptions>| f(debug_options.as_ref()))
}

pub fn show_debug_options(
    ui: &mut Ui,
    debug_options: &mut DebugOptions,
) -> egui::CollapsingResponse<()> {
    egui::CollapsingHeader::new("Debugging Options").show(ui, |ui| {
        egui::ComboBox::from_label("Debug grid")
            .selected_text(debug_options.debug_grid.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut debug_options.debug_grid,
                    GridSize::None,
                    GridSize::None.to_string(),
                );
                ui.selectable_value(
                    &mut debug_options.debug_grid,
                    GridSize::Eight,
                    GridSize::Eight.to_string(),
                );
                ui.selectable_value(
                    &mut debug_options.debug_grid,
                    GridSize::Four,
                    GridSize::Four.to_string(),
                );
                ui.selectable_value(
                    &mut debug_options.debug_grid,
                    GridSize::Two,
                    GridSize::Two.to_string(),
                );
            });
    })
}

fn setup(mut commands: Commands) {
    commands.spawn(DebugGrid);
}

pub fn bevy_inspector_ui(ui: InMut<Ui>, world: &mut World) {
    egui::ScrollArea::both().show(ui.0, |ui| {
        bevy_inspector::ui_for_world(world, ui);
    });
}

pub fn egui_inspector_ui(ui: InMut<Ui>, mut contexts: EguiContexts) {
    contexts.ctx_mut().inspection_ui(ui.0);
}

fn render_grid(
    frame: Res<Frame>,
    debug_options: Res<DebugOptions>,
    mut gizmos: Gizmos,
    mut debug_grid: Query<(&mut Transform, &InheritedVisibility), With<DebugGrid>>,
) {
    let (transform, visibility) = debug_grid.single_mut();
    if !visibility.get() {
        return;
    }
    if debug_options.debug_grid >= GridSize::Two {
        let grid_cells_2 = frame.size / 2;
        gizmos.grid_2d(
            transform.translation.xy(),
            grid_cells_2,
            FRAME_ASPECT_RATIO * transform.scale.xy() / grid_cells_2.as_vec2(),
            css::GREEN,
        );
    }
    if debug_options.debug_grid >= GridSize::Four {
        let grid_cells_4 = frame.size / 4;
        gizmos.grid_2d(
            transform.translation.xy(),
            grid_cells_4,
            FRAME_ASPECT_RATIO * transform.scale.xy() / grid_cells_4.as_vec2(),
            css::BLUE,
        );
    }
    if debug_options.debug_grid >= GridSize::Eight {
        let grid_cells_8 = frame.size / 8;
        gizmos.grid_2d(
            transform.translation.xy(),
            grid_cells_8,
            FRAME_ASPECT_RATIO * transform.scale.xy() / grid_cells_8.as_vec2(),
            css::RED,
        );
    }
}

struct MemoryState {
    bytes_per_row: usize,
    last_start: usize,
    last_memory: Vec<u8>,
    animation_countdowns: Vec<u8>,
}

pub fn memory_ui(ui: InMut<Ui>, machine: Option<Res<Machine>>, mut bytes_per_row: Local<usize>) {
    if *bytes_per_row == 0 {
        *bytes_per_row = 8;
    }
    let memory = machine
        .as_ref()
        .map(|machine| machine.machine.memory())
        .unwrap_or(&[]);
    let num_rows = if memory.is_empty() {
        0
    } else {
        (memory.len() - 1) / *bytes_per_row + 1
    };

    ui.0.horizontal(|ui| {
        ui.label("Bytes per row:");
        ui.selectable_value(&mut *bytes_per_row, 8, "8");
        ui.selectable_value(&mut *bytes_per_row, 16, "16");
        ui.selectable_value(&mut *bytes_per_row, 32, "32");
    });

    ui.0.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        egui::ScrollArea::both().auto_shrink(false).show_rows(
            ui,
            ui.text_style_height(&egui::TextStyle::Body),
            num_rows,
            |ui, rows| {
                for (i, chunk) in memory
                    .chunks(*bytes_per_row)
                    .enumerate()
                    .skip(rows.start)
                    .take(rows.end - rows.start)
                {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:#06X}", i * *bytes_per_row));
                        for (j, byte) in chunk.iter().enumerate() {
                            ui.colored_label(
                                if j % 2 == 0 {
                                    style::FOREGROUND_LIGHT
                                } else {
                                    style::FOREGROUND_MID
                                },
                                format!("{:02X}", byte),
                            );
                        }
                    });
                }
            },
        );
    });
}
