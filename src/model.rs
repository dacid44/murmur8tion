use rand::SeedableRng;

use crate::{hardware::KeyEvent, screen};

pub trait Model {
    type Screen: screen::Screen;
    type Rng: rand::Rng;
    fn init_screen(&self) -> Self::Screen;
    fn init_rng(&self) -> Self::Rng;
    fn quirks(&self) -> &Quirks;
}

#[derive(Debug, Clone, Copy)]
pub struct Quirks {
    pub bitshift_use_y: bool,
    pub key_wait_trigger: KeyEvent,
    pub inc_i_on_slice: bool,
    pub bitwise_reset_flags: bool,
    pub draw_wait_for_vblank: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CosmacVip;

impl CosmacVip {
    const QUIRKS: Quirks = Quirks {
        bitshift_use_y: true,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: true,
        bitwise_reset_flags: true,
        draw_wait_for_vblank: true,
    };
}

impl Model for CosmacVip {
    type Screen = screen::CosmacVipScreen;
    type Rng = rand_xoshiro::Xoshiro256PlusPlus;

    fn init_screen(&self) -> Self::Screen {
        Default::default()
    }

    fn init_rng(&self) -> Self::Rng {
        Self::Rng::from_os_rng()
    }

    fn quirks(&self) -> &Quirks {
        &Self::QUIRKS
    }
}
