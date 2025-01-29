use rand::SeedableRng;

use crate::{hardware::KeyEvent, instruction::InstructionSet, screen};

pub trait Model {
    type Screen: screen::Screen;
    type Rng: rand::Rng;
    fn init_screen(&self) -> Self::Screen;
    fn init_rng(&self) -> Self::Rng;
    fn instruction_set(&self) -> InstructionSet;
    fn quirks(&self) -> &Quirks;
}

#[derive(Debug, Clone, Copy)]
pub struct Quirks {
    pub bitshift_use_y: bool,
    pub key_wait_trigger: KeyEvent,
    pub inc_i_on_slice: bool,
    pub bitwise_reset_flag: bool,
    pub draw_wait_for_vblank: bool,
    pub clear_screen_on_mode_switch: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        CosmacVip::QUIRKS
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CosmacVip;

impl CosmacVip {
    const QUIRKS: Quirks = Quirks {
        bitshift_use_y: true,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: true,
        bitwise_reset_flag: true,
        draw_wait_for_vblank: true,
        clear_screen_on_mode_switch: false,
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

    fn instruction_set(&self) -> InstructionSet {
        InstructionSet::CosmacVip
    }

    fn quirks(&self) -> &Quirks {
        &Self::QUIRKS
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModernSchip;

impl ModernSchip {
    const QUIRKS: Quirks = Quirks {
        bitshift_use_y: true,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: true,
        bitwise_reset_flag: false,
        draw_wait_for_vblank: true,
        clear_screen_on_mode_switch: true,
    };
}

impl Model for ModernSchip {
    type Screen = screen::SuperChipScreen;
    type Rng = rand_xoshiro::Xoshiro256PlusPlus;

    fn init_screen(&self) -> Self::Screen {
        Default::default()
    }

    fn init_rng(&self) -> Self::Rng {
        Self::Rng::from_os_rng()
    }

    fn instruction_set(&self) -> InstructionSet {
        InstructionSet::SuperChip
    }

    fn quirks(&self) -> &Quirks {
        &Self::QUIRKS
    }
}
