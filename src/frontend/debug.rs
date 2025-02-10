use std::fmt::Display;

use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Ui},
    EguiContexts,
};
use bevy_inspector_egui::bevy_inspector;
use range_vec::RangeVec;

use crate::hardware::Machine as HardwareMachine;

use super::{layout::ScaleToDisplay, machine::Machine, ui::style, Frame, FRAME_ASPECT_RATIO};

#[derive(Resource, Clone, Default)]
pub struct DebugOptions {
    debug_grid: GridSize,
}

#[derive(Component)]
#[require(ScaleToDisplay(|| ScaleToDisplay(FRAME_ASPECT_RATIO)), Transform, InheritedVisibility)]
pub struct DebugGrid;

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
        .add_systems(Startup, setup);
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

pub fn render_grid_egui(
    ui: InMut<Ui>,
    frame: Res<Frame>,
    debug_options: Res<DebugOptions>,
    debug_grid: Query<(&ScaleToDisplay, &Transform), With<DebugGrid>>,
) {
    if debug_options.debug_grid == GridSize::None {
        return;
    }

    let (ratio, transform) = debug_grid.single();
    let painter = ui.painter();
    let rect = egui::Rect::from_center_size(
        painter.clip_rect().center(),
        (transform.scale.xy() * ratio.0).to_array().into(),
    );
    let grid_spacing = ratio.0 * transform.scale.xy() / frame.size.as_vec2();

    // CSS green
    let green = egui::Color32::from_rgb(0, 128, 0);

    for i in 1..frame.size.x {
        if i % 8 == 0 {
            painter.vline(
                rect.left() + (i as f32 * grid_spacing.x),
                rect.y_range(),
                (2.0, egui::Color32::RED),
            );
        } else if debug_options.debug_grid >= GridSize::Four && i % 4 == 0 {
            painter.vline(
                rect.left() + (i as f32 * grid_spacing.x),
                rect.y_range(),
                (2.0, egui::Color32::BLUE),
            );
        } else if debug_options.debug_grid >= GridSize::Two && i % 2 == 0 {
            painter.vline(
                rect.left() + (i as f32 * grid_spacing.x),
                rect.y_range(),
                (2.0, green),
            );
        }
    }

    for i in 1..frame.size.y {
        if i % 8 == 0 {
            painter.hline(
                rect.x_range(),
                rect.top() + (i as f32 * grid_spacing.y),
                (2.0, egui::Color32::RED),
            );
        } else if debug_options.debug_grid >= GridSize::Four && i % 4 == 0 {
            painter.hline(
                rect.x_range(),
                rect.top() + (i as f32 * grid_spacing.y),
                (2.0, egui::Color32::BLUE),
            );
        } else if debug_options.debug_grid >= GridSize::Two && i % 2 == 0 {
            painter.hline(
                rect.x_range(),
                rect.top() + (i as f32 * grid_spacing.y),
                (2.0, green),
            );
        }
    }
}

pub struct MemoryState {
    bytes_per_row: usize,
    last_memory: RangeVec<u8>,
    change_counters: RangeVec<u8>,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            bytes_per_row: 8,
            last_memory: RangeVec::new(),
            change_counters: RangeVec::new(),
        }
    }
}

impl MemoryState {
    fn update(&mut self, new_memory: &[u8], new_start: usize) {
        self.change_counters
            .mutate_non_default(|_, counter| *counter -= 1);
        let new_range = new_start..new_start + new_memory.len();
        self.last_memory.truncate(new_range.clone());
        if let Some(range) = self.last_memory.range() {
            for (i, (last, new)) in self
                .last_memory
                .iter(range.clone())
                .zip(new_memory[range.start - new_start..].iter())
                .enumerate()
            {
                if new != last {
                    self.change_counters.set(i + range.start, 30);
                }
            }
        }
        self.last_memory.as_mut_slices(new_range, |left, right| {
            left.copy_from_slice(&new_memory[..left.len()]);
            right.copy_from_slice(&new_memory[left.len()..]);
        });
    }

    fn get_counter(&self, row: usize, col: usize) -> u8 {
        *self.change_counters.get(row * self.bytes_per_row + col)
    }
}

pub fn memory_ui(ui: InMut<Ui>, machine: Option<Res<Machine>>, mut state: Local<MemoryState>) {
    let memory = machine
        .as_ref()
        .map(|machine| machine.machine.memory())
        .unwrap_or(&[]);
    let num_rows = if memory.is_empty() {
        0
    } else {
        (memory.len() - 1) / state.bytes_per_row + 1
    };

    ui.0.horizontal(|ui| {
        ui.label("Bytes per row:");
        ui.selectable_value(&mut state.bytes_per_row, 8, "8");
        ui.selectable_value(&mut state.bytes_per_row, 16, "16");
        ui.selectable_value(&mut state.bytes_per_row, 32, "32");
    });

    ui.0.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        egui::ScrollArea::both().auto_shrink(false).show_rows(
            ui,
            ui.text_style_height(&egui::TextStyle::Body),
            num_rows,
            |ui, rows| {
                let range = rows.start * state.bytes_per_row..rows.end * state.bytes_per_row;
                state.update(&memory[range.clone()], range.start);

                for (i, chunk) in memory
                    .chunks(state.bytes_per_row)
                    .enumerate()
                    .skip(rows.start)
                    .take(rows.end - rows.start)
                {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:#06X}", i * state.bytes_per_row));
                        for (j, byte) in chunk.iter().enumerate() {
                            let base_color = if j % 2 == 0 {
                                style::FOREGROUND_LIGHT
                            } else {
                                style::FOREGROUND_MID
                            };
                            let color = match state.get_counter(i, j) {
                                counter @ ..=30 => base_color
                                    .lerp_to_gamma(style::ACCENT_LIGHT, counter as f32 / 30.0),
                                _ => base_color,
                            };
                            ui.colored_label(color, format!("{:02X}", byte));
                        }
                    });
                }

                let range_start = range.start;
                state.update(&memory[range], range_start);
            },
        );
    });
}
