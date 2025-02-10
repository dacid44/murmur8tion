use bevy::{prelude::*, winit::WinitSettings};
use murmur8tion::{model::Model, *};

// fn setup_global_subscriber() -> impl Drop {
//     use std::{fs::File, io::BufWriter};
//     use tracing_flame::FlameLayer;
//     use tracing_subscriber::{fmt, prelude::*, registry::Registry};

//     let fmt_layer = fmt::Layer::default();

//     let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

//     let subscriber = Registry::default().with(fmt_layer).with(flame_layer);

//     tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
//     _guard
// }

fn main() {
    // let tracing_flame_guard = setup_global_subscriber();
    // puffin::set_scopes_on(true);

    println!("Hello, world!");

    App::new()
        .insert_resource(WinitSettings::game())
        .insert_resource(Time::<Fixed>::from_hz(
            model::DynamicModel::default().default_framerate(),
        ))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Murmur8tion".to_owned(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_plugins(frontend::emulator_plugin)
        .run();
}
