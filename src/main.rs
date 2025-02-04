use bevy::{prelude::*, winit::WinitSettings};
use model::Model;

mod frontend;
mod hardware;
mod instruction;
mod model;
mod screen;

fn main() {
    println!("Hello, world!");

    App::new()
        .insert_resource(WinitSettings::game())
        .insert_resource(Time::<Fixed>::from_hz(model::CosmacVip.default_framerate()))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "CHIP-8 Emulator".to_owned(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_plugins(frontend::emulator_plugin)
        .run();
}
