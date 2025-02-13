use std::{
    collections::BTreeSet,
    f32,
    fmt::{Display, UpperHex},
    ops::{Sub, SubAssign},
};

use bevy::prelude::*;
use bevy_egui::{
    egui::{self, style::ScrollAnimation, Ui, WidgetText},
    EguiContexts,
};
use bevy_inspector_egui::bevy_inspector;
use range_vec::RangeVec;

use crate::{
    hardware::{self, Machine as HardwareMachine},
    instruction::{ExecuteInstruction, InstructionSet, OctoSyntax},
    model::{CosmacVip, Quirks},
};

use super::{
    layout::ScaleToDisplay,
    machine::{Machine, ToMachine},
    ui::style,
    EmulatorData, EmulatorEvent, Frame, FRAME_ASPECT_RATIO,
};

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

#[derive(Default)]
pub struct DebuggerState {
    last_pc: u16,
    scroll_offset: f32,
    is_odd: Option<bool>,
    breakpoints: BTreeSet<usize>,
}

pub fn debugger_ui(
    ui: InMut<Ui>,
    machine: Res<Machine>,
    mut emulator_data: ResMut<EmulatorData>,
    mut emulator_events: EventWriter<EmulatorEvent>,
    mut state: Local<DebuggerState>,
) {
    ui.0.horizontal(|ui| {
        if large_button(ui, "▶", false, !emulator_data.paused)
            .on_hover_text("Resume")
            .clicked()
        {
            emulator_data.paused = false;
        }
        if large_button(ui, "⏸", false, emulator_data.paused)
            .on_hover_text("Pause")
            .clicked()
        {
            emulator_data.paused = true;
        }

        if large_button(ui, "»", true, false)
            .on_hover_text("Next Instruction")
            .clicked()
        {
            machine.tx.try_send(ToMachine::Step).unwrap();
        }

        if large_button(ui, "⟲", true, false)
            .on_hover_text("Reset")
            .clicked()
        {
            emulator_events.send(EmulatorEvent::ResetMachine);
        }
    });

    ui.0.horizontal(|ui| {
        ui.label("Parity:");
        ui.selectable_value(&mut state.is_odd, Some(false), "Even");
        ui.selectable_value(&mut state.is_odd, Some(true), "Odd");
        ui.selectable_value(&mut state.is_odd, None, "PC");
    });

    if ui.0.button("Clear all breakpoints").clicked() {
        state.breakpoints.clear();
        machine.tx.try_send(ToMachine::ClearBreakpoints).unwrap();
    }

    let memory = machine.machine.memory();
    let pc = machine.machine.cpu().pc;
    let quirks = machine.machine.quirks();
    let instruction_set = machine.machine.instruction_set();

    let pc_usize = pc as usize;
    let is_odd = state.is_odd.unwrap_or(pc % 2 == 1);
    let num_rows = (memory.len() / 2).saturating_sub(is_odd as usize);
    let text_height = ui.0.text_style_height(&egui::TextStyle::Body);

    ui.0.group(|ui| {
        state.scroll_offset = egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show_rows(ui, text_height, num_rows, |ui, rows| {
                let spacing = ui.style().spacing.item_spacing.x;

                for row in rows {
                    let address = row * 2 + (is_odd as usize);
                    let pc_color = if pc_usize == address {
                        Some(style::ACCENT_LIGHT)
                    } else if address + 1 == pc_usize || pc_usize + 1 == address {
                        Some(style::ACCENT_MID)
                    } else {
                        None
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(
                            pc_color.unwrap_or(style::FOREGROUND_MID),
                            format!("{address:04X}:"),
                        );
                        if let Some(breakpoint) =
                            breakpoint_button(ui, &mut state.breakpoints, address).inner
                        {
                            machine
                                .tx
                                .try_send(ToMachine::SetBreakpoint(address as u16, breakpoint))
                                .unwrap();
                        }

                        let color = pc_color.unwrap_or(style::FOREGROUND_LIGHT);
                        if let Some(OpcodeInfo {
                            opcode,
                            is_long_operand,
                            long_operand,
                            instruction,
                        }) = get_opcode(memory, address, quirks, instruction_set)
                        {
                            let color = if is_long_operand {
                                style::NEUTRAL_MID
                            } else {
                                color
                            };
                            ui.colored_label(
                                color,
                                match long_operand {
                                    Some(addr) => format!("{opcode:04X} {addr:04X}"),
                                    None => format!("{opcode:04X}     "),
                                },
                            );
                            ui.add_space(spacing * 2.0);
                            ui.colored_label(color, instruction);
                        }
                    });
                }

                if pc != state.last_pc {
                    let scroll_row = if pc == 0 {
                        pc_usize + (pc % 2 == 1 && is_odd) as usize
                    } else {
                        pc_usize - (pc % 2 == 1 && is_odd) as usize
                    } / 2;
                    let top = (text_height + ui.style().spacing.item_spacing.y) * scroll_row as f32
                        - state.scroll_offset;
                    let bottom = top + text_height;

                    ui.scroll_to_rect_animation(
                        egui::Rect::from_x_y_ranges(ui.clip_rect().x_range(), top..=bottom),
                        Some(egui::Align::Center),
                        ScrollAnimation::none(),
                    );

                    state.last_pc = pc;
                }
            })
            .state
            .offset
            .y;
    });
}

fn breakpoint_button(
    ui: &mut Ui,
    breakpoints: &mut BTreeSet<usize>,
    address: usize,
) -> egui::InnerResponse<Option<bool>> {
    let selected = breakpoints.contains(&address);

    let painter = ui.painter();
    let font = egui::FontId::new(
        egui::TextStyle::Button.resolve(ui.style()).size,
        egui::FontFamily::Name("Pixel Code SlightlyRaised".into()),
    );
    let wrap_width = ui.available_width();

    let desired_size = painter
        .layout(
            " ".to_owned(),
            font.clone(),
            egui::Color32::TRANSPARENT,
            wrap_width,
        )
        .size();
    // desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click());

    let (text, color) = match (
        selected,
        response.is_pointer_button_down_on(),
        response.hovered(),
    ) {
        (false, false, false) => (" ", egui::Color32::TRANSPARENT),
        (false, false, true) => ("○", style::FOREGROUND_LIGHT),
        (false, true, _) => ("○", style::ACCENT_MID),
        (true, false, false) => ("●", style::ACCENT_MID),
        (true, false, true) => ("●", style::FOREGROUND_LIGHT),
        (true, true, _) => ("○", style::ACCENT_MID),
    };

    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            text,
        )
    });

    if ui.is_rect_visible(response.rect) {
        let text_pos = ui.layout().align_size_within_rect(desired_size, rect).min;
        ui.painter()
            .text(text_pos, egui::Align2::LEFT_TOP, text, font, color);
    }

    egui::InnerResponse::new(
        if response.clicked() {
            if selected {
                breakpoints.remove(&address);
                Some(false)
            } else {
                breakpoints.insert(address);
                Some(true)
            }
        } else {
            None
        },
        response,
    )
}

struct OpcodeInfo {
    opcode: u16,
    is_long_operand: bool,
    long_operand: Option<u16>,
    instruction: String,
}

fn get_opcode(
    memory: &[u8],
    address: usize,
    quirks: &Quirks,
    instruction_set: InstructionSet,
) -> Option<OpcodeInfo> {
    let last_word = address
        .checked_sub(2)
        .and_then(|addr| memory.get(addr).zip(memory.get(addr + 1)))
        .map(|(left, right)| u16::from_be_bytes([*left, *right]));
    let word = u16::from_be_bytes([*memory.get(address)?, *memory.get(address + 1)?]);
    let next_word = memory
        .get(address + 2)
        .zip(memory.get(address + 3))
        .map(|(left, right)| u16::from_be_bytes([*left, *right]));

    let (is_long_operand, last_can_skip) = last_word
        .map(|last_word| {
            let mut last_parser = OctoSyntax(quirks, Some(word));
            let last_can_skip = last_parser
                .execute(last_word, instruction_set)
                .is_some_and(|last_instruction| last_instruction.ends_with("then"));
            (last_parser.1.is_none(), last_can_skip)
        })
        .unwrap_or((false, false));

    let mut parser = OctoSyntax(quirks, next_word);
    let Some(mut instruction) = parser.execute(word, instruction_set) else {
        return Some(OpcodeInfo {
            opcode: word,
            is_long_operand,
            long_operand: None,
            instruction: "????".to_owned(),
        });
    };

    if last_can_skip {
        instruction.insert_str(0, "    ");
    }

    Some(OpcodeInfo {
        opcode: word,
        is_long_operand,
        long_operand: parser.1.xor(next_word),
        instruction,
    })
}

fn large_button(
    ui: &mut Ui,
    label: impl Into<String>,
    slightly_raised_text: bool,
    selected: bool,
) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(label)
                .family(if slightly_raised_text {
                    egui::FontFamily::Name("Pixel Code SlightlyRaised".into())
                } else {
                    egui::FontFamily::Name("Pixel Code Raised".into())
                })
                .size(egui::TextStyle::Button.resolve(ui.style()).size * 2.0),
        )
        .min_size(style::LARGE_BUTTON_SIZE)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .selected(selected),
    )
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

pub struct Counters(hardware::Cpu);

impl Default for Counters {
    fn default() -> Self {
        Self(hardware::Cpu {
            pc: 0,
            ..Default::default()
        })
    }
}

pub fn registers_ui(
    ui: InMut<Ui>,
    machine: Option<Res<Machine>>,
    mut last_cpu: Local<hardware::Cpu>,
    mut counters: Local<Counters>,
) {
    let cpu = machine
        .map(|machine| machine.machine.cpu().clone())
        .unwrap_or_default();
    let counters = &mut counters.0;

    ui.0.vertical(|ui| {
        egui::Grid::new("cpu_v_registers")
            .spacing(ui.style().spacing.item_spacing * egui::vec2(2.0, 1.0))
            .show(ui, |ui| {
                for i in 0..16 {
                    let reg = (i % 4 * 4) + (i / 4);
                    show_register(
                        ui,
                        format!("v{:X}", reg),
                        2,
                        cpu.v[reg],
                        &mut last_cpu.v[reg],
                        &mut counters.v[reg],
                    );
                    if i % 4 == 3 {
                        ui.end_row();
                    }
                }
            });

        ui.add_space(ui.style().spacing.item_spacing.y);
        show_register(ui, "PC:", 4, cpu.pc, &mut last_cpu.pc, &mut counters.pc);

        ui.add_space(ui.style().spacing.item_spacing.y);
        show_register(ui, "I:", 4, cpu.i, &mut last_cpu.i, &mut counters.i);

        ui.add_space(ui.style().spacing.item_spacing.y);
        show_register(ui, "DT:", 2, cpu.dt, &mut last_cpu.dt, &mut counters.dt);
        show_register(ui, "ST:", 2, cpu.st, &mut last_cpu.st, &mut counters.st);
    });
}

fn show_register<V>(
    ui: &mut Ui,
    label: impl Into<egui::RichText>,
    digits: usize,
    value: V,
    last_value: &mut V,
    counter: &mut V,
) where
    V: Copy + Display + UpperHex + Eq + Ord + SubAssign + From<u8> + Into<f32>,
{
    if value != *last_value {
        *counter = 30.into();
    } else if *counter > 0.into() {
        *counter -= 1.into();
    }
    // println!("value: {value}, last_value: {last_value}, counter: {counter}");
    *last_value = value;
    let color =
        style::FOREGROUND_LIGHT.lerp_to_gamma(style::ACCENT_LIGHT, (*counter).into() / 30.0);

    ui.horizontal(|ui| {
        ui.colored_label(style::FOREGROUND_MID, label);
        ui.colored_label(color, format!("{1:00$X}", digits, value));
    });
}
