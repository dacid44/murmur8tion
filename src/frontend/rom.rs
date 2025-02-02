use bevy::{
    prelude::*,
    tasks::{block_on, poll_once, IoTaskPool, Task},
};

use super::{EmulatorData, EmulatorEvent};

#[derive(Component)]
struct PickRom(Task<Option<(String, Vec<u8>)>>);

#[derive(Resource)]
pub struct Rom(pub Vec<u8>);

pub fn rom_plugin(app: &mut App) {
    app.add_systems(Update, rom_loaded.run_if(any_with_component::<PickRom>))
        .add_systems(PostUpdate, start_pick_rom.run_if(on_event::<EmulatorEvent>));
}

fn start_pick_rom(mut commands: Commands, mut ui_events: EventReader<EmulatorEvent>) {
    for event in ui_events.read() {
        if matches!(event, EmulatorEvent::PickRom) {
            let task = IoTaskPool::get().spawn(async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Choose a ROM file")
                    .add_filter("Chip-8 ROMs", &["ch8", "xo8"])
                    .pick_file()
                    .await?;

                match async_fs::read(file.path()).await {
                    Ok(data) => Some((
                        file.path()
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "..".to_owned()),
                        data,
                    )),
                    Err(error) => {
                        error!(
                            "Error reading chosen file {}: {}",
                            file.path().display(),
                            error
                        );
                        None
                    }
                }
            });
            commands.spawn(PickRom(task));
        }
    }
}

fn rom_loaded(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut PickRom)>,
    mut ui_data: ResMut<EmulatorData>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(maybe_rom) = block_on(poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            if let Some(rom) = maybe_rom {
                ui_data.rom_name = Some(rom.0);
                commands.insert_resource(Rom(rom.1));
            }
        }
    }
}
