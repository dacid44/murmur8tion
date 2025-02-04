use std::fmt::Display;

use rand::SeedableRng;

use crate::{
    hardware::KeyEvent,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Display for DrawWaitSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrawWaitSetting::Always => write!(f, "Always"),
            DrawWaitSetting::LoresOnly => write!(f, "Lores mode only"),
            DrawWaitSetting::Never => write!(f, "Never"),
        }
    }
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
                Self::CosmacVip(model) => Model::$name(model$(, $param)*),
                Self::LegacySuperChip(model) => Model::$name(model$(, $param)*),
                Self::ModernSuperChip(model) => Model::$name(model$(, $param)*),
                Self::XoChip(model) => Model::$name(model$(, $param)*),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicModel {
    CosmacVip(CosmacVip),
    LegacySuperChip(LegacySuperChip),
    ModernSuperChip(ModernSuperChip),
    XoChip(XoChip),
}

impl Default for DynamicModel {
    fn default() -> Self {
        Self::CosmacVip(CosmacVip::default())
    }
}

impl Display for DynamicModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CosmacVip(_) => write!(f, "COSMAC VIP"),
            Self::LegacySuperChip(_) => write!(f, "Legacy SUPER-CHIP (SUPER-CHIP 1.1)"),
            Self::ModernSuperChip(_) => write!(f, "Modern SUPER-CHIP (Octo)"),
            Self::XoChip(_) => write!(f, "XO-CHIP"),
        }
    }
}

impl Model for DynamicModel {
    type Screen = DynamicScreen;
    type Rng = Box<dyn rand::RngCore>;

    fn init_screen(&self) -> Self::Screen {
        match self {
            Self::CosmacVip(_) => DynamicScreen::CosmacVip(Default::default()),
            Self::LegacySuperChip(_) => DynamicScreen::LegacySuperChip(Default::default()),
            Self::ModernSuperChip(_) => DynamicScreen::ModernSuperChip(Default::default()),
            Self::XoChip(_) => DynamicScreen::XoChip(Default::default()),
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

impl DynamicModel {
    pub const COSMAC_VIP: Self = Self::CosmacVip(CosmacVip(CosmacVip::QUIRKS));
    pub const LEGACY_SCHIP: Self = Self::LegacySuperChip(LegacySuperChip(LegacySuperChip::QUIRKS));
    pub const MODERN_SCHIP: Self = Self::ModernSuperChip(ModernSuperChip(ModernSuperChip::QUIRKS));
    pub const XO_CHIP: Self = Self::XoChip(XoChip(XoChip::QUIRKS));

    pub fn quirks_mut(&mut self) -> &mut Quirks {
        match self {
            Self::CosmacVip(CosmacVip(quirks)) => quirks,
            Self::LegacySuperChip(LegacySuperChip(quirks)) => quirks,
            Self::ModernSuperChip(ModernSuperChip(quirks)) => quirks,
            Self::XoChip(XoChip(quirks)) => quirks,
        }
    }

    pub fn default_quirks(&self) -> Quirks {
        match self {
            Self::CosmacVip(_) => CosmacVip::QUIRKS,
            Self::LegacySuperChip(_) => LegacySuperChip::QUIRKS,
            Self::ModernSuperChip(_) => ModernSuperChip::QUIRKS,
            Self::XoChip(_) => XoChip::QUIRKS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CosmacVip(pub Quirks);

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

impl Default for CosmacVip {
    fn default() -> Self {
        Self(Self::QUIRKS)
    }
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
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacySuperChip(pub Quirks);

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

impl Default for LegacySuperChip {
    fn default() -> Self {
        Self(Self::QUIRKS)
    }
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
        &self.0
    }

    fn default_framerate(&self) -> f64 {
        64.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModernSuperChip(pub Quirks);

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

impl Default for ModernSuperChip {
    fn default() -> Self {
        Self(Self::QUIRKS)
    }
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
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XoChip(pub Quirks);

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

impl Default for XoChip {
    fn default() -> Self {
        Self(Self::QUIRKS)
    }
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
        &self.0
    }
}
