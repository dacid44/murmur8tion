use std::fmt::Display;

use arbitrary_int::{u4, Number};
use bevy::log::warn;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use thiserror::Error;

use crate::{
    frontend::audio::DEFAULT_PATTERN,
    instruction::{ExecuteInstruction, InstructionSet},
    match_execute,
    model::{self, CosmacVip, DynamicModel, LegacySuperChip, ModernSuperChip, Quirks, XoChip},
    screen::{
        self, CosmacVipScreen, LegacySuperChipScreen, ModernSuperChipScreen, Palette, XoChipScreen,
    },
};

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("invalid instruction '{0:#06X}'")]
    InvalidInstruction(u16),
    #[error("ret was called with no return value on the stack")]
    StackEmpty,
    #[error("call was called when the stack was full")]
    StackFull,
    #[error("an invalid memory range was accessed (range {range} of memory size {memory_size:#X})", range = format_range(*start, *offset, *inclusive))]
    InvalidMemoryRange {
        start: u16,
        offset: usize,
        inclusive: bool,
        memory_size: usize,
    },
    #[error("an unsupported screen operation was run")]
    UnsupportedScreenOperation(#[from] screen::UnsupportedScreenOperation),
}

fn format_range(start: u16, offset: usize, inclusive: bool) -> String {
    let end = (start as usize) + offset;
    if inclusive {
        format!("{start:#06X}..={end:#06X}")
    } else {
        format!("{start:#06X}..{end:#06X}")
    }
}

pub trait Machine: Send + Sync {
    fn event(&mut self, key: u4, event: KeyEvent);
    fn render_frame(&self, palette: &Palette) -> image::RgbaImage;
    fn tick_timers(&mut self);
    fn disable_vblank(&mut self);
    fn sound_active(&self) -> bool;
    fn pitch(&self) -> u8;
    fn audio_pattern(&self) -> &[u8; 16];
    fn memory(&self) -> &[u8];
    fn cpu(&self) -> &Cpu;
    fn quirks(&self) -> &Quirks;
    fn instruction_set(&self) -> InstructionSet;
    fn tick(&mut self) -> Result<bool>;
    fn tick_many(&mut self, count: u32) -> Result<bool> {
        self.tick()?;
        self.disable_vblank();
        for _ in 1..count {
            self.tick()?;
            // coz::progress!("machine_tick")
            // if self.tick()? {
            //     return Ok(true)
            // }
        }
        Ok(false)
    }
}

macro_rules! blanket_machine_method {
    ($name:ident(self: $($selfty:ty)?$(, $param:ident: $ptype:ty)*)$( -> $ret:ty)?) => {
        fn $name(self$(: $selfty)?$(, $param: $ptype)*)$( -> $ret)? {
            Chip8::$name(self$(, $param)*)
        }
    }
}

impl<Model, Screen> Machine for Chip8<Model, Screen>
where
    Model: model::Model,
    Screen: screen::Screen,
{
    blanket_machine_method!(event(self: &mut Self, key: u4, event: KeyEvent));
    blanket_machine_method!(render_frame(self: &Self, palette: &Palette) -> image::RgbaImage);
    blanket_machine_method!(tick_timers(self: &mut Self));
    blanket_machine_method!(disable_vblank(self: &mut Self));
    blanket_machine_method!(sound_active(self: &Self) -> bool);
    blanket_machine_method!(pitch(self: &Self) -> u8);
    blanket_machine_method!(audio_pattern(self: &Self) -> &[u8; 16]);
    blanket_machine_method!(memory(self: &Self) -> &[u8]);
    blanket_machine_method!(cpu(self: &Self) -> &Cpu);
    blanket_machine_method!(quirks(self: &Self) -> &Quirks);
    blanket_machine_method!(instruction_set(self: &Self) -> InstructionSet);
    blanket_machine_method!(tick(self: &mut Self) -> Result<bool>);
}

type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! dynamic_machine_method {
    ($name:ident(self: $($selfty:ty)?$(, $param:ident: $ptype:ty)*)$( -> $ret:ty)?) => {
        fn $name(self$(: $selfty)?$(, $param: $ptype)*)$( -> $ret)? {
            match self {
                Self::CosmacVip(machine) => Chip8::$name(machine$(, $param)*),
                Self::LegacySuperChip(machine) => Chip8::$name(machine$(, $param)*),
                Self::ModernSuperChip(machine) => Chip8::$name(machine$(, $param)*),
                Self::XoChip(machine) => Chip8::$name(machine$(, $param)*),
            }
        }
    }
}

#[derive(Clone)]
pub enum DynamicMachine {
    CosmacVip(Chip8<CosmacVip, CosmacVipScreen>),
    LegacySuperChip(Chip8<LegacySuperChip, LegacySuperChipScreen>),
    ModernSuperChip(Chip8<ModernSuperChip, ModernSuperChipScreen>),
    XoChip(Chip8<XoChip, XoChipScreen>),
}

impl DynamicMachine {
    pub fn new(model: DynamicModel, rom: &[u8]) -> Self {
        match model {
            DynamicModel::CosmacVip(model) => Self::new_cosmac_vip(model, rom),
            DynamicModel::LegacySuperChip(model) => Self::new_legacy_schip(model, rom),
            DynamicModel::ModernSuperChip(model) => Self::new_modern_schip(model, rom),
            DynamicModel::XoChip(model) => Self::new_xochip(model, rom),
        }
    }

    pub fn new_cosmac_vip(model: CosmacVip, rom: &[u8]) -> Self {
        Self::CosmacVip(Chip8::new(model, Box::<CosmacVipScreen>::default(), rom))
    }

    pub fn new_legacy_schip(model: LegacySuperChip, rom: &[u8]) -> Self {
        Self::LegacySuperChip(Chip8::new(
            model,
            Box::<LegacySuperChipScreen>::default(),
            rom,
        ))
    }

    pub fn new_modern_schip(model: ModernSuperChip, rom: &[u8]) -> Self {
        Self::ModernSuperChip(Chip8::new(
            model,
            Box::<ModernSuperChipScreen>::default(),
            rom,
        ))
    }

    pub fn new_xochip(model: XoChip, rom: &[u8]) -> Self {
        Self::XoChip(Chip8::new(model, Box::<XoChipScreen>::default(), rom))
    }
}

impl Machine for DynamicMachine {
    dynamic_machine_method!(event(self: &mut Self, key: u4, event: KeyEvent));
    dynamic_machine_method!(render_frame(self: &Self, palette: &Palette) -> image::RgbaImage);
    dynamic_machine_method!(tick_timers(self: &mut Self));
    dynamic_machine_method!(disable_vblank(self: &mut Self));
    dynamic_machine_method!(sound_active(self: &Self) -> bool);
    dynamic_machine_method!(pitch(self: &Self) -> u8);
    dynamic_machine_method!(audio_pattern(self: &Self) -> &[u8; 16]);
    dynamic_machine_method!(memory(self: &Self) -> &[u8]);
    dynamic_machine_method!(cpu(self: &Self) -> &Cpu);
    dynamic_machine_method!(quirks(self: &Self) -> &Quirks);
    dynamic_machine_method!(instruction_set(self: &Self) -> InstructionSet);
    dynamic_machine_method!(tick(self: &mut Self) -> Result<bool>);
    dynamic_machine_method!(tick_many(self: &mut Self, count: u32) -> Result<bool>);
}

#[derive(Clone)]
pub struct Cpu {
    pub v: [u8; 16],
    pub i: u16,
    pub dt: u8,
    pub st: u8,
    pub pc: u16,
    pub sp: u4,
    pub stack: [u16; 16],
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            v: [0; 16],
            i: 0,
            dt: 0,
            st: 0,
            pc: 0x200,
            sp: u4::MIN,
            stack: [0; 16],
        }
    }
}

impl Cpu {
    fn get_v(&self, reg: u4) -> u8 {
        // SAFETY: reg is a u4 and therefore cannot be larger than 15
        // unsafe { *self.v.get_unchecked(reg.value() as usize) }
        self.v[reg.value() as usize]
    }

    fn set_v(&mut self, reg: u4, val: u8) {
        // SAFETY: reg is a u4 and therefore cannot be larger than 15
        // unsafe {
        //     *self.v.get_unchecked_mut(reg.value() as usize) = val;
        // }
        self.v[reg.value() as usize] = val;
    }

    fn push_stack(&mut self) -> Result<()> {
        self.sp = self.sp.checked_add(u4::new(1)).ok_or(Error::StackFull)?;
        self.stack[self.sp.value() as usize] = self.pc;
        Ok(())
    }

    fn pop_stack(&mut self) -> Result<()> {
        match self.sp.checked_sub(u4::new(1)) {
            Some(sp) => {
                self.pc = self.stack[self.sp.value() as usize];
                self.sp = sp;
            }
            None => return Err(Error::StackEmpty),
        }
        Ok(())
    }

    fn inc_pc(&mut self) {
        self.pc = self.pc.wrapping_add(2)
    }

    fn dec_pc(&mut self) {
        self.pc -= 2;
    }

    fn arithmetic_op(&mut self, x: u4, y: u4, f: impl FnOnce(u8, u8) -> u8, reset_vf: bool) {
        self.set_v(x, f(self.get_v(x), self.get_v(y)));
        if reset_vf {
            self.v[0xF] = 0;
        }
    }

    fn arithmetic_op_vf(&mut self, x: u4, y: u4, f: impl FnOnce(u8, u8) -> (u8, bool)) {
        let (result, vf) = f(self.get_v(x), self.get_v(y));
        self.set_v(x, result);
        self.v[0xF] = vf as u8;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Keypad {
    keys: u16,
    waiting: bool,
    event: Option<u4>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Press,
    Release,
}

impl Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyEvent::Press => write!(f, "Press"),
            KeyEvent::Release => write!(f, "Release"),
        }
    }
}

impl Keypad {
    fn event(&mut self, key: u4, event: KeyEvent, test_event: KeyEvent) {
        let was_pressed = self.keys & 1 << u8::from(key) != 0;
        let record_event = match event {
            KeyEvent::Press => {
                self.keys |= 1 << u8::from(key);
                !was_pressed
            }
            KeyEvent::Release => {
                self.keys &= !(1 << u8::from(key));
                was_pressed
            }
        };
        if record_event && event == test_event {
            self.event = Some(self.event.unwrap_or(key).min(key));
        }
    }

    fn test_event(&mut self) -> Option<u4> {
        match (self.waiting, self.event) {
            (true, Some(key)) => {
                self.waiting = false;
                Some(key)
            }
            (true, None) => None,
            (false, _) => {
                self.waiting = true;
                self.event = None;
                None
            }
        }
    }

    fn is_pressed(&self, key: u8) -> bool {
        self.keys & 1 << (key & 0x0F) != 0
    }
}

#[derive(Clone)]
pub struct Chip8<Model: model::Model, Screen: screen::Screen + ?Sized> {
    model: Model,
    keypad: Keypad,
    cpu: Cpu,
    memory: Box<[u8]>,
    screen: Box<Screen>,
    rng: Xoshiro256PlusPlus,
    vblank: bool,
    rpl: [u8; 16],
    pitch: u8,
    audio_pattern: [u8; 16],
}

impl<Model: model::Model, Screen: screen::Screen + ?Sized> Chip8<Model, Screen> {
    pub fn new(model: Model, screen: Box<Screen>, rom: &[u8]) -> Self {
        let memory_size = model.memory_size();
        let mut memory = bytemuck::zeroed_slice_box(memory_size);
        let font_slice: &[u8] = screen::FONT.as_flattened();
        memory[screen::FONT_ADDRESS..screen::FONT_ADDRESS + font_slice.len()]
            .copy_from_slice(font_slice);
        let hires_font_slice: &[u8] = screen::XOCHIP_HIRES_FONT.as_flattened();
        memory[screen::XOCHIP_HIRES_FONT_ADDRESS
            ..screen::XOCHIP_HIRES_FONT_ADDRESS + hires_font_slice.len()]
            .copy_from_slice(hires_font_slice);
        if let Some(slice) = memory.get_mut(0x200..0x200 + rom.len()) {
            slice.copy_from_slice(rom);
        } else {
            warn!("ROM is too big to completely load into memory");
            memory[0x200..].copy_from_slice(&rom[..memory_size - 0x200]);
        }
        Self {
            keypad: Default::default(),
            model,
            cpu: Default::default(),
            memory,
            screen,
            rng: Xoshiro256PlusPlus::from_os_rng(),
            vblank: false,
            rpl: [0; 16],
            pitch: 64,
            audio_pattern: DEFAULT_PATTERN,
        }
    }

    pub fn event(&mut self, key: u4, event: KeyEvent) {
        self.keypad
            .event(key, event, self.model.quirks().key_wait_trigger)
    }

    pub fn render_frame(&self, palette: &Palette) -> image::RgbaImage {
        self.screen.to_image(palette)
    }

    pub fn tick_timers(&mut self) {
        if self.cpu.dt > 0 {
            self.cpu.dt -= 1;
        }
        if self.cpu.st > 0 {
            self.cpu.st -= 1;
        }
        self.vblank = true;
    }

    pub fn disable_vblank(&mut self) {
        self.vblank = false;
    }

    pub fn sound_active(&self) -> bool {
        self.cpu.st > 0
    }

    pub fn pitch(&self) -> u8 {
        self.pitch
    }

    pub fn audio_pattern(&self) -> &[u8; 16] {
        &self.audio_pattern
    }

    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn quirks(&self) -> &Quirks {
        self.model.quirks()
    }

    pub fn instruction_set(&self) -> InstructionSet {
        self.model.instruction_set()
    }

    fn draw_wait_for_vblank(&self) -> bool {
        self.model
            .quirks()
            .draw_wait_for_vblank
            .wait(self.screen.get_hires())
    }

    fn skip_if(&mut self, condition: bool) -> Result<()> {
        if condition {
            if self.model.instruction_set() >= InstructionSet::XoChip && self.read_word()? == 0xF000
            {
                self.cpu.inc_pc();
            }
            self.cpu.inc_pc();
        }
        Ok(())
    }

    fn read_word(&self) -> Result<u16> {
        let pc = self.cpu.pc as usize;
        match (self.memory.get(pc), self.memory.get(pc.wrapping_add(1))) {
            (Some(high), Some(low)) => Ok(u16::from_be_bytes([*high, *low])),
            _ => Err(Error::InvalidMemoryRange {
                start: self.cpu.pc,
                offset: 2,
                inclusive: false,
                memory_size: self.memory.len(),
            }),
        }
    }

    // Returns a boolean specifying whether to exit
    pub fn tick(&mut self) -> Result<bool> {
        let instruction = self.read_word()?;
        self.cpu.inc_pc();
        self.execute(instruction, self.model.instruction_set())
    }
}

impl<Model: model::Model, Screen: screen::Screen + ?Sized> ExecuteInstruction<Result<bool>>
    for Chip8<Model, Screen>
{
    match_execute! { Result<bool>, self, x, y, n, x_u8, y_u8, n_u8, nn, nnn; Ok(false);
        _0000 => {
            if self.model.quirks().graceful_exit_on_0000 {
                return Ok(true);
            } else {
                return Err(Error::InvalidInstruction(0x0000));
            }
        }
        _00Cn => {
            self.screen.scroll_down(n)?;
        }
        _00Dn => {
            self.screen.scroll_up(n)?;
        }
        _00E0 => {
            self.screen.clear();
        }
        _00EE => {
            self.cpu.pop_stack()?;
        }
        _00FB => {
            self.screen.scroll_right()?;
        }
        _00FC => {
            self.screen.scroll_left()?;
        }
        _00FD => {
            return Ok(true);
        }
        _00FE => {
            self.screen.set_hires(false)?;
            if self.model.quirks().clear_screen_on_mode_switch {
                self.screen.clear();
            }
        }
        _00FF => {
            self.screen.set_hires(true)?;
            if self.model.quirks().clear_screen_on_mode_switch {
                self.screen.clear();
            }
        }
        _1nnn => {
            self.cpu.pc = nnn;
        }
        _2nnn => {
            self.cpu.push_stack()?;
            self.cpu.pc = nnn;
        }
        _3xnn => {
            self.skip_if(self.cpu.get_v(x) == nn)?;
        }
        _4xnn => {
            self.skip_if(self.cpu.get_v(x) != nn)?;
        }
        _5xy0 => {
            self.skip_if(self.cpu.get_v(x) == self.cpu.get_v(y))?;
        }
        _5xy2 => {
            let x_usize = u8::from(x) as usize;
            let y_usize = u8::from(y) as usize;
            let mem_slice = mem_slice_inclusive_mut(
                &mut self.memory,
                self.cpu.i,
                x_usize.abs_diff(y_usize),
            )?;
            if y_usize >= x_usize {
                mem_slice.copy_from_slice(&self.cpu.v[x_usize..=y_usize]);
            } else {
                mem_slice.copy_from_slice(&self.cpu.v[y_usize..=x_usize]);
                mem_slice.reverse();
            }
        }
        _5xy3 => {
            let x_usize = u8::from(x) as usize;
            let y_usize = u8::from(y) as usize;
            let mem_slice =
                mem_slice_inclusive(&self.memory, self.cpu.i, x_usize.abs_diff(y_usize))?;
            if y_usize >= x_usize {
                self.cpu.v[x_usize..=y_usize].copy_from_slice(mem_slice);
            } else {
                self.cpu.v[y_usize..=x_usize].copy_from_slice(mem_slice);
                self.cpu.v[y_usize..=x_usize].reverse();
            }
        }
        _6xnn => {
            self.cpu.set_v(x, nn);
        }
        _7xnn => {
            self.cpu.set_v(x, self.cpu.get_v(x).wrapping_add(nn));
        }
        _8xy0 => {
            self.cpu.set_v(x, self.cpu.get_v(y));
        }
        _8xy1 => {
            self.cpu.arithmetic_op(
                x,
                y,
                std::ops::BitOr::bitor,
                self.model.quirks().bitwise_reset_flag,
            );
        }
        _8xy2 => {
            self.cpu.arithmetic_op(
                x,
                y,
                std::ops::BitAnd::bitand,
                self.model.quirks().bitwise_reset_flag,
            );
        }
        _8xy3 => {
            self.cpu.arithmetic_op(
                x,
                y,
                std::ops::BitXor::bitxor,
                self.model.quirks().bitwise_reset_flag,
            );
        }
        _8xy4 => {
            self.cpu.arithmetic_op_vf(x, y, u8::overflowing_add);
        }
        _8xy5 => {
            self.cpu.arithmetic_op_vf(x, y, |a, b| {
                let (result, borrow) = a.overflowing_sub(b);
                (result, !borrow)
            });
        }
        _8xy6 => {
            self.cpu.arithmetic_op_vf(
                x,
                if self.model.quirks().bitshift_use_y {
                    y
                } else {
                    x
                },
                |_, b| {
                    let overflow_bit = b & 0b1 != 0;
                    (b >> 1, overflow_bit)
                },
            );
        }
        _8xy7 => {
            self.cpu.arithmetic_op_vf(x, y, |a, b| {
                let (result, borrow) = b.overflowing_sub(a);
                (result, !borrow)
            });
        }
        _8xyE => {
            self.cpu.arithmetic_op_vf(
                x,
                if self.model.quirks().bitshift_use_y {
                    y
                } else {
                    x
                },
                |_, b| {
                    let overflow_bit = b & 0b10000000 != 0;
                    (b << 1, overflow_bit)
                },
            );
        }
        _9xy0 => {
            self.skip_if(self.cpu.get_v(x) != self.cpu.get_v(y))?;
        }
        _Annn => {
            self.cpu.i = nnn;
        }
        _Bnnn => {
            let offset = self.cpu.get_v(if self.model.quirks().jump_v0_use_vx {
                x
            } else {
                u4::new(0)
            });
            self.cpu.pc = nnn + offset as u16;
        }
        _Cxnn => {
            self.cpu.set_v(x, self.rng.random::<u8>() & nn);
        }
        _Dxy0 => {
            if self.draw_wait_for_vblank() && !self.vblank {
                self.cpu.dec_pc();
            } else {
                let x_val = self.cpu.get_v(x);
                let y_val = self.cpu.get_v(y);
                if self.model.quirks().lores_draw_large_as_small && !self.screen.get_hires() {
                    self.cpu.v[0xF] = self.screen.draw_sprite(
                        x_val,
                        y_val,
                        mem_slice(
                            &self.memory,
                            self.cpu.i,
                            16 * self.screen.num_active_planes(),
                        )?,
                    ) as u8;
                } else {
                    self.cpu.v[0xF] = self.screen.draw_large_sprite(
                        x_val,
                        y_val,
                        bytemuck::cast_slice(mem_slice(
                            &self.memory,
                            self.cpu.i,
                            32 * self.screen.num_active_planes(),
                        )?),
                    )?;
                }
            }
        }
        _Dxyn => {
            if self.draw_wait_for_vblank() && !self.vblank {
                self.cpu.dec_pc();
            } else {
                let x_val = self.cpu.get_v(x);
                let y_val = self.cpu.get_v(y);
                self.cpu.v[0xF] = self.screen.draw_sprite(
                    x_val,
                    y_val,
                    mem_slice(
                        &self.memory,
                        self.cpu.i,
                        n_u8 as usize * self.screen.num_active_planes(),
                    )?,
                ) as u8;
            }
        }
        _Ex9E => {
            self.skip_if(self.keypad.is_pressed(self.cpu.get_v(x)))?;
        }
        _ExA1 => {
            self.skip_if(!self.keypad.is_pressed(self.cpu.get_v(x)))?;
        }
        _F000 => {
            let addr = self.read_word()?;
            self.cpu.inc_pc();
            self.cpu.i = addr;
        }
        _Fx01 => {
            self.screen.set_planes(x)?;
        }
        _F002 => {
            self.audio_pattern
                .copy_from_slice(mem_slice(&self.memory, self.cpu.i, 16)?);
        }
        _Fx07 => {
            self.cpu.set_v(x, self.cpu.dt);
        }
        _Fx0A => {
            if let Some(key) = self.keypad.test_event() {
                self.cpu.set_v(x, u8::from(key));
            } else {
                self.cpu.dec_pc();
            }
        }
        _Fx15 => {
            self.cpu.dt = self.cpu.get_v(x);
        }
        _Fx18 => {
            self.cpu.st = self.cpu.get_v(x);
        }
        _Fx1E => {
            self.cpu.i = self.cpu.i.wrapping_add(self.cpu.get_v(x) as u16);
        }
        _Fx29 => {
            self.cpu.i = ((self.cpu.get_v(x) & 0xF) * screen::FONT[0].len() as u8) as u16
                + screen::FONT_ADDRESS as u16;
        }
        _Fx30 => {
            self.cpu.i = ((self.cpu.get_v(x) & 0xF) * screen::XOCHIP_HIRES_FONT[0].len() as u8)
                as u16
                + screen::XOCHIP_HIRES_FONT_ADDRESS as u16;
        }
        _Fx33 => {
            mem_slice_mut(&mut self.memory, self.cpu.i, 3)?
                .copy_from_slice(&bcd(self.cpu.get_v(x)));
        }
        _Fx3A => {
            self.pitch = self.cpu.get_v(x);
        }
        _Fx55 => {
            mem_slice_inclusive_mut(&mut self.memory, self.cpu.i, x_u8 as usize)?
                .copy_from_slice(&self.cpu.v[..=x_u8 as usize]);
            if self.model.quirks().inc_i_on_slice {
                self.cpu.i = self.cpu.i.wrapping_add(x_u8 as u16).wrapping_add(1);
            }
        }
        _Fx65 => {
            self.cpu.v[..=x_u8 as usize].copy_from_slice(mem_slice_inclusive(
                &self.memory,
                self.cpu.i,
                x_u8 as usize,
            )?);
            if self.model.quirks().inc_i_on_slice {
                self.cpu.i = self.cpu.i.wrapping_add(x_u8 as u16).wrapping_add(1);
            }
        }
        _Fx75 => {
            self.rpl[..=u8::from(x) as usize]
                .copy_from_slice(&self.cpu.v[..=u8::from(x) as usize]);
        }
        _Fx85 => {
            self.cpu.v[..=u8::from(x) as usize]
                .copy_from_slice(&self.rpl[..=u8::from(x) as usize]);
        }
    }

    fn no_match(
        &mut self,
        instruction: u16,
        _x: u4,
        _y: u4,
        _n: u4,
        _x_u8: u8,
        _y_u8: u8,
        _n_u8: u8,
        _nn: u8,
        _nnn: u16,
    ) -> Result<bool> {
        self.cpu.dec_pc();
        Err(Error::InvalidInstruction(instruction))
    }
}

fn bcd(x: u8) -> [u8; 3] {
    [x / 100, x / 10 % 10, x % 10]
}

fn mem_slice(memory: &[u8], start: u16, offset: usize) -> Result<&[u8]> {
    match memory.get(start as usize..(start as usize).wrapping_add(offset)) {
        Some(slice) => Ok(slice),
        None => Err(Error::InvalidMemoryRange {
            start,
            offset,
            inclusive: false,
            memory_size: memory.len(),
        }),
    }
}

fn mem_slice_inclusive(memory: &[u8], start: u16, offset: usize) -> Result<&[u8]> {
    match memory.get(start as usize..=(start as usize).wrapping_add(offset)) {
        Some(slice) => Ok(slice),
        None => Err(Error::InvalidMemoryRange {
            start,
            offset,
            inclusive: true,
            memory_size: memory.len(),
        }),
    }
}

fn mem_slice_mut(memory: &mut [u8], start: u16, offset: usize) -> Result<&mut [u8]> {
    let memory_len = memory.len();
    match memory.get_mut(start as usize..(start as usize).wrapping_add(offset)) {
        Some(slice) => Ok(slice),
        None => Err(Error::InvalidMemoryRange {
            start,
            offset,
            inclusive: false,
            memory_size: memory_len,
        }),
    }
}

fn mem_slice_inclusive_mut(memory: &mut [u8], start: u16, offset: usize) -> Result<&mut [u8]> {
    let memory_len = memory.len();
    match memory.get_mut(start as usize..=(start as usize).wrapping_add(offset)) {
        Some(slice) => Ok(slice),
        None => Err(Error::InvalidMemoryRange {
            start,
            offset,
            inclusive: true,
            memory_size: memory_len,
        }),
    }
}
