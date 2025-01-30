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
    pub lores_draw_large_as_small: bool,
}

impl Default for Quirks {
    fn default() -> Self {
        CosmacVip::QUIRKS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawWaitSetting {
    Always,
    LoresOnly,
    Never,
}

impl DrawWaitSetting {
    pub fn wait(&self, hires: bool) -> bool {
        match self {
            DrawWaitSetting::Always => true,
            DrawWaitSetting::LoresOnly => !hires,
            DrawWaitSetting::Never => false,
        }
    }
}

macro_rules! dynamic_model_method {
    ($name:ident(self: $($selfty:ty)?$(, $param:ident: $ptype:ty)*)$( -> $ret:ty)?) => {
        fn $name(self$(: $selfty)?$(, $param: $ptype)*)$( -> $ret)? {
            match self {
                Self::CosmacVip => CosmacVip.$name($($param), *),
                Self::LegacySuperChip => LegacySuperChip.$name($($param), *),
                Self::ModernSuperChip => ModernSuperChip.$name($($param), *),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicModel {
    CosmacVip,
    LegacySuperChip,
    ModernSuperChip,
}

impl Display for DynamicModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CosmacVip => write!(f, "COSMAC VIP"),
            Self::LegacySuperChip => write!(f, "Legacy SUPER-CHIP (SUPER-CHIP 1.1)"),
            Self::ModernSuperChip => write!(f, "Modern SUPER-CHIP (Octo)"),
        }
    }
}

impl Model for DynamicModel {
    type Screen = DynamicScreen;
    type Rng = Box<dyn rand::RngCore>;

    fn init_screen(&self) -> Self::Screen {
        match self {
            Self::CosmacVip => DynamicScreen::CosmacVip(Default::default()),
            Self::LegacySuperChip => DynamicScreen::LegacySuperChip(Default::default()),
            Self::ModernSuperChip => DynamicScreen::ModernSuperChip(Default::default()),
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
        draw_wait_for_vblank: DrawWaitSetting::Always,
        clear_screen_on_mode_switch: false,
        jump_v0_use_vx: false,
        lores_draw_large_as_small: true,
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
pub struct LegacySuperChip;

impl LegacySuperChip {
    const QUIRKS: Quirks = Quirks {
        bitshift_use_y: false,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: false,
        bitwise_reset_flag: false,
        draw_wait_for_vblank: DrawWaitSetting::LoresOnly,
        clear_screen_on_mode_switch: false,
        jump_v0_use_vx: true,
        lores_draw_large_as_small: true,
    };
}

impl Model for LegacySuperChip {
    type Screen = screen::LegacySuperChipScreen;
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

#[derive(Debug, Clone, Copy)]
pub struct ModernSuperChip;

impl ModernSuperChip {
    const QUIRKS: Quirks = Quirks {
        bitshift_use_y: false,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: false,
        bitwise_reset_flag: false,
        draw_wait_for_vblank: DrawWaitSetting::Never,
        clear_screen_on_mode_switch: true,
        jump_v0_use_vx: true,
        lores_draw_large_as_small: false,
    };
}

impl Model for ModernSuperChip {
    type Screen = screen::ModernSuperChipScreen;
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
