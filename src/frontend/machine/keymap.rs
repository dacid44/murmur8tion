use arbitrary_int::u4;
use bevy::{ecs::system::Resource, input::keyboard::KeyCode, utils::HashMap};

#[derive(Resource)]
pub struct KeyMapping {
    pub keys: HashMap<KeyCode, u4>,
}

const DEFAULT_KEY_MAPPING: [KeyCode; 16] = [
    KeyCode::KeyX,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::KeyQ,
    KeyCode::KeyW,
    KeyCode::KeyE,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::KeyZ,
    KeyCode::KeyC,
    KeyCode::Digit4,
    KeyCode::KeyR,
    KeyCode::KeyF,
    KeyCode::KeyV,
];

impl Default for KeyMapping {
    fn default() -> Self {
        Self {
            keys: DEFAULT_KEY_MAPPING
                .iter()
                .enumerate()
                .map(|(i, key)| (*key, u4::from_u8(i as u8)))
                .collect(),
        }
    }
}
