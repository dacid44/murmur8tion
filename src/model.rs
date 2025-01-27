use rand::SeedableRng;

use crate::{hardware::KeyEvent, screen};

pub trait Model {
    type Screen: screen::Screen;
    type Rng: rand::Rng;
    fn init_screen(&self) -> Self::Screen;
    fn init_rng(&self) -> Self::Rng;
    fn bitshift_use_y(&self) -> bool;
    fn key_wait_trigger(&self) -> KeyEvent;
}

struct CosmacVip;

impl Model for CosmacVip {
    type Screen = screen::CosmacVipScreen;
    type Rng = rand_xoshiro::Xoshiro256PlusPlus;

    fn init_screen(&self) -> Self::Screen {
        Default::default()
    }

    fn init_rng(&self) -> Self::Rng {
        Self::Rng::from_os_rng()
    }

    fn bitshift_use_y(&self) -> bool {
        true
    }

    fn key_wait_trigger(&self) -> KeyEvent {
        KeyEvent::Release
    }
}
