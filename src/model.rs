use std::fmt::Display;

use rand::SeedableRng;

use crate::{
    hardware::{Chip8, KeyEvent},
    instruction::InstructionSet,
    screen::{self, DynamicScreen},
};

pub trait Model {
    type Screen: screen::Screen;
    type Rng: rand::Rng;
    fn init_screen(&self) -> Self::Screen;
    fn init_rng(&self) -> Self::Rng;
    fn instruction_set(&self) -> InstructionSet;
    fn quirks(&self) -> &Quirks {
        &CosmacVip::QUIRKS
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Quirks {
    pub bitshift_use_y: bool,
    pub key_wait_trigger: KeyEvent,
    pub inc_i_on_slice: bool,
    pub bitwise_reset_flag: bool,
    pub draw_wait_for_vblank: DrawWaitSetting,
    pub clear_screen_on_mode_switch: bool,
    pub jump_v0_use_vx: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        CosmacVip::QUIRKS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawWaitSetting {
    On,
    LoresOnly,
    Off,
}

impl DrawWaitSetting {
    pub fn wait(&self, hires: bool) -> bool {
        match self {
            DrawWaitSetting::On => true,
            DrawWaitSetting::LoresOnly => !hires,
            DrawWaitSetting::Off => false,
        }
    }
}

macro_rules! dynamic_model_method {
    ($name:ident(self: $($selfty:ty)?$(, $param:ident: $ptype:ty)*)$( -> $ret:ty)?) => {
        fn $name(self$(: $selfty)?$(, $param: $ptype)*)$( -> $ret)? {
            match self {
                Self::CosmacVip => CosmacVip.$name($($param), *),
                Self::ModernSchip => ModernSchip.$name($($param), *),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicModel {
    CosmacVip,
    ModernSchip,
}

impl Display for DynamicModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CosmacVip => write!(f, "COSMAC VIP"),
            Self::ModernSchip => write!(f, "Modern SUPER-CHIP"),
        }
    }
}

impl Model for DynamicModel {
    type Screen = DynamicScreen;
    type Rng = Box<dyn rand::RngCore>;

    fn init_screen(&self) -> Self::Screen {
        match self {
            Self::CosmacVip => DynamicScreen::CosmacVip(Default::default()),
            Self::ModernSchip => DynamicScreen::SuperChip(Default::default()),
        }
    }

    fn init_rng(&self) -> Self::Rng {
        Box::new(rand_xoshiro::Xoshiro256PlusPlus::from_os_rng())
    }

    dynamic_model_method!(instruction_set(self: &Self) -> InstructionSet);
    dynamic_model_method!(quirks(self: &Self) -> &Quirks);
}

#[derive(Debug, Clone, Copy)]
pub struct CosmacVip;

impl CosmacVip {
    const QUIRKS: Quirks = Quirks {
        bitshift_use_y: true,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: true,
        bitwise_reset_flag: true,
        draw_wait_for_vblank: DrawWaitSetting::On,
        clear_screen_on_mode_switch: false,
        jump_v0_use_vx: false,
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
        bitshift_use_y: false,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: false,
        bitwise_reset_flag: false,
        draw_wait_for_vblank: DrawWaitSetting::LoresOnly,
        clear_screen_on_mode_switch: true,
        jump_v0_use_vx: true,
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
