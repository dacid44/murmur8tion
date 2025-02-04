use std::fmt::Display;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{
    egui::{self, Color32, Ui},
    EguiContext, EguiPlugin,
};
use egui_tiles::{Container, Linear, LinearDir, SimplificationOptions, Tile, TileId, Tiles, Tree};

use super::{
    debug::bevy_inspector_ui,
    ui::{draw_main_ui, style},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmulatorTab {
    Main,
    Display,
    BevyInspector,
}

impl Display for EmulatorTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmulatorTab::Main => write!(f, "Main"),
            EmulatorTab::Display => write!(f, "Display"),
            EmulatorTab::BevyInspector => write!(f, "Bevy Inspector"),
        }
    }
}

struct Behavior<'a> {
    world: &'a mut World,
    display_rect: &'a mut Option<egui::Rect>,
    available_panes: &'a mut Vec<EmulatorTab>,
    add_pane: &'a mut Option<(TileId, EmulatorTab)>,
}

impl egui_tiles::Behavior<EmulatorTab> for Behavior<'_> {
    fn pane_ui(
        &mut self,
        ui: &mut Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut EmulatorTab,
    ) -> egui_tiles::UiResponse {
        let background_color = match pane {
            EmulatorTab::Display => egui::Color32::TRANSPARENT,
            _ => ui.style().visuals.panel_fill,
        };
        egui::Frame::central_panel(ui.style())
            .fill(background_color)
            .show(ui, |ui| {
                match pane {
                    EmulatorTab::Display => *self.display_rect = Some(ui.clip_rect()),
                    EmulatorTab::Main => {
                        self.world
                            .run_system_cached_with(draw_main_ui, ui)
                            .expect("failed to draw main UI tab");
                    }
                    EmulatorTab::BevyInspector => {
                        self.world
                            .run_system_cached_with(bevy_inspector_ui, ui)
                            .expect("failed to draw bevy inspector UI");
                    }
                }
                ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            });

        Default::default()
    }

    fn tab_title_for_pane(&mut self, pane: &EmulatorTab) -> bevy_egui::egui::WidgetText {
        pane.to_string().into()
    }

    fn is_tab_closable(&self, tiles: &Tiles<EmulatorTab>, tile_id: TileId) -> bool {
        match tiles.get_pane(&tile_id) {
            Some(EmulatorTab::Main | EmulatorTab::Display) | None => false,
            Some(_) => true,
        }
    }

    fn on_tab_close(&mut self, tiles: &mut Tiles<EmulatorTab>, tile_id: TileId) -> bool {
        recursive_find_panes(self.available_panes, tiles, tile_id);
        self.available_panes.sort_unstable();
        true
    }

    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn tab_bar_color(&self, _visuals: &egui::Visuals) -> egui::Color32 {
        style::BACKGROUND_NEUTRAL
    }

    fn tab_bg_color(
        &self,
        visuals: &egui::Visuals,
        tiles: &Tiles<EmulatorTab>,
        tile_id: TileId,
        state: &egui_tiles::TabState,
    ) -> egui::Color32 {
        if state.active {
            if tiles.get_pane(&tile_id) == Some(&EmulatorTab::Display) {
                style::BACKGROUND_DARK
            } else {
                visuals.panel_fill
            }
        } else {
            Color32::TRANSPARENT
        }
    }

    // fn tab_text_color(
    //     &self,
    //     _visuals: &egui::Visuals,
    //     _tiles: &Tiles<EmulatorTab>,
    //     _tile_id: TileId,
    //     state: &egui_tiles::TabState,
    // ) -> egui::Color32 {
    //     match (state.is_being_dragged, state.active) {
    //         (true, _) => style::ACCENT_DARK,
    //         (false, true) => style::FOREGROUND_LIGHT,
    //         (false, false) => style::ACCENT_DARK,
    //     }
    // }

    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<EmulatorTab>,
        ui: &mut Ui,
        tile_id: TileId,
        _tabs: &egui_tiles::Tabs,
        _scroll_offset: &mut f32,
    ) {
        ui.add_space(ui.spacing().item_spacing.x / 4.0);
        ui.menu_button("➕", |ui| {
            if !self.available_panes.is_empty() {
                for pane in self.available_panes.iter() {
                    if ui.button(pane.to_string()).clicked() {
                        *self.add_pane = Some((tile_id, *pane));
                    }
                }
            } else {
                ui.add_enabled(false, egui::Label::new("No more panes"));
            }
        });
    }
}

fn recursive_find_panes<Pane: Clone>(panes: &mut Vec<Pane>, tiles: &Tiles<Pane>, tile_id: TileId) {
    match tiles.get(tile_id) {
        Some(Tile::Pane(pane)) => panes.push(pane.clone()),
        Some(Tile::Container(container)) => {
            for child in container.children() {
                recursive_find_panes(panes, tiles, *child);
            }
        }
        None => {}
    }
}

#[derive(Resource, Default)]
struct DisplayRect(Option<Rect>);

#[derive(Resource)]
struct Layout {
    tree: Tree<EmulatorTab>,
    available_panes: Vec<EmulatorTab>,
}

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct ScaleToDisplay(pub Vec2);

pub fn layout_plugin(app: &mut App) {
    app.add_plugins(EguiPlugin)
        .init_resource::<DisplayRect>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_ui)
        .add_systems(PostUpdate, scale_display);
}

fn setup(mut commands: Commands) {
    let mut tiles = Tiles::default();
    let main = tiles.insert_pane(EmulatorTab::Main);
    let display = tiles.insert_pane(EmulatorTab::Display);

    let mut root_container = Linear::new(LinearDir::Horizontal, vec![main, display]);
    root_container.shares.set_share(main, 1.0);
    root_container.shares.set_share(display, 3.0);
    let root = tiles.insert_container(Container::Linear(root_container));
    let tree = Tree::new("layout", root, tiles);

    commands.insert_resource(Layout {
        tree,
        available_panes: vec![EmulatorTab::BevyInspector],
    });
}

fn draw_ui(world: &mut World) {
    let mut egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .single(world)
        .clone();
    let mut new_display_rect = None;
    let mut add_pane = None;

    world.resource_scope::<Layout, _>(|world, mut layout| {
        let Layout {
            ref mut tree,
            ref mut available_panes,
        } = layout.as_mut();
        let mut behavior = Behavior {
            world,
            display_rect: &mut new_display_rect,
            available_panes,
            add_pane: &mut add_pane,
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_context.get_mut(), |ui| {
                tree.ui(&mut behavior, ui);
            });

        if let Some((tile_id, pane)) = add_pane {
            let new_child = tree.tiles.insert_pane(pane);
            if let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get_mut(tile_id) {
                tabs.add_child(new_child);
                tabs.set_active(new_child);
                available_panes.retain(|p| *p != pane);
            } else {
                panic!("could not find tab layout to add tabs to");
            }
        }
    });

    let new_display_rect = new_display_rect.map(egui_to_bevy_rect);
    let mut display_rect = world.resource_mut::<DisplayRect>();
    if new_display_rect != display_rect.0 {
        display_rect.0 = new_display_rect;
    }
}

fn scale_display(
    display_rect: Res<DisplayRect>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut display_transforms: Query<(&mut Transform, &mut Visibility, &ScaleToDisplay)>,
) {
    let window_size = window.single().size();
    if let Some(display_rect) = display_rect.0 {
        let new_transform = (display_rect.center() - window_size / 2.0) * Vec2::new(1.0, -1.0);
        for (mut transform, mut visibility, ratio) in display_transforms.iter_mut() {
            let scale = (display_rect.size() / ratio.0).min_element();
            transform.translation.x = new_transform.x;
            transform.translation.y = new_transform.y;
            transform.scale = Vec3::new(scale, scale, 1.0);
            *visibility = Visibility::Inherited;
        }
    } else {
        for (_, mut visibility, _) in display_transforms.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }
}

fn egui_to_bevy_rect(rect: bevy_egui::egui::Rect) -> Rect {
    Rect::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y)
}
