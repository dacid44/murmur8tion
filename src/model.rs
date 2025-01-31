use std::{fmt::Display, ops::DerefMut};

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
    fn memory_size(&self) -> usize {
        0x1000
    }
    fn instruction_set(&self) -> InstructionSet;
    fn quirks(&self) -> &Quirks {
        &CosmacVip::QUIRKS
    }
    fn default_framerate(&self) -> f64 {
        60.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Quirks {
    pub graceful_exit_on_0000: bool,
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
                Self::XoChip => XoChip.$name($($param), *),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicModel {
    CosmacVip,
    LegacySuperChip,
    ModernSuperChip,
    XoChip,
}

impl Display for DynamicModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CosmacVip => write!(f, "COSMAC VIP"),
            Self::LegacySuperChip => write!(f, "Legacy SUPER-CHIP (SUPER-CHIP 1.1)"),
            Self::ModernSuperChip => write!(f, "Modern SUPER-CHIP (Octo)"),
            Self::XoChip => write!(f, "XO-CHIP"),
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
            Self::XoChip => DynamicScreen::XoChip(Default::default()),
        }
    }

    fn init_rng(&self) -> Self::Rng {
        Box::new(rand_xoshiro::Xoshiro256PlusPlus::from_os_rng())
    }

    dynamic_model_method!(memory_size(self: &Self) -> usize);
    dynamic_model_method!(instruction_set(self: &Self) -> InstructionSet);
    dynamic_model_method!(quirks(self: &Self) -> &Quirks);
    dynamic_model_method!(default_framerate(self: &Self) -> f64);
}

#[derive(Debug, Clone, Copy)]
pub struct CosmacVip;

impl CosmacVip {
    const QUIRKS: Quirks = Quirks {
        graceful_exit_on_0000: false,
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
        graceful_exit_on_0000: false,
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

    fn default_framerate(&self) -> f64 {
        64.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModernSuperChip;

impl ModernSuperChip {
    const QUIRKS: Quirks = Quirks {
        graceful_exit_on_0000: false,
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

#[derive(Debug, Clone, Copy)]
pub struct XoChip;

impl XoChip {
    const QUIRKS: Quirks = Quirks {
        graceful_exit_on_0000: false,
        bitshift_use_y: true,
        key_wait_trigger: KeyEvent::Release,
        inc_i_on_slice: true,
        bitwise_reset_flag: false,
        draw_wait_for_vblank: DrawWaitSetting::Never,
        clear_screen_on_mode_switch: true,
        jump_v0_use_vx: false,
        lores_draw_large_as_small: false,
    };
}

impl Model for XoChip {
    type Screen = screen::XoChipScreen;
    type Rng = rand_xoshiro::Xoshiro256PlusPlus;

    fn init_screen(&self) -> Self::Screen {
        Default::default()
    }

    fn init_rng(&self) -> Self::Rng {
        Self::Rng::from_os_rng()
    }

    fn memory_size(&self) -> usize {
        0x10000
    }

    fn instruction_set(&self) -> InstructionSet {
        InstructionSet::XoChip
    }

    fn quirks(&self) -> &Quirks {
        &Self::QUIRKS
    }
}
