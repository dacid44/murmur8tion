use std::{
    collections::VecDeque,
    fmt::Display,
    ops::{Bound, RangeBounds},
    slice::SliceIndex,
};

use arbitrary_int::{u4, Number};
use bevy::log::{info_span, warn};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;
use thiserror::Error;

use crate::{
    frontend::audio::DEFAULT_PATTERN,
    instruction::{Args, Instruction, InstructionSet, SuperChipInstruction, XoChipInstruction},
    model::{self, CosmacVip, DynamicModel, LegacySuperChip, Model, ModernSuperChip, XoChip},
    screen::{
        self, CosmacVipScreen, LegacySuperChipScreen, ModernSuperChipScreen, Palette, Screen,
        XoChipScreen,
    },
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid instruction '{0:#06X}'")]
    InvalidInstruction(u16),
    #[error("encountered an instruction not supported by this CHIP-8 model ({0:?})")]
    UnsupportedInstruction(Instruction),
    #[error("the program counter tried to add past available memory (address {0:#07X})")]
    PcOverflow(u32),
    #[error("ret was called with no return value on the stack")]
    StackEmpty,
    #[error("call was called when the stack was full")]
    StackFull,
    #[error("an invalid memory range was accessed (range {0:#X?} of memory size {1:#X})")]
    InvalidMemoryRange((Bound<usize>, Bound<usize>), usize),
    #[error("an invalid exact memory range was accessed (address {address:#X} of length {length:#X} in memory size {memory_size:#X}")]
    InvalidExactMemoryRange {
        address: usize,
        length: usize,
        memory_size: usize,
    },
    #[error("an unsupported screen operation was run")]
    UnsupportedScreenOperation(#[from] screen::UnsupportedScreenOperation),
}

pub trait Machine: Send + Sync {
    fn event(&mut self, key: u4, event: KeyEvent);
    fn render_frame(&mut self, palette: &Palette) -> image::RgbaImage;
    fn sound_active(&self) -> bool;
    fn pitch(&self) -> u8;
    fn audio_pattern(&self) -> &[u8; 16];
    fn memory(&self) -> &[u8];
    fn tick(&mut self) -> Result<bool>;
    fn tick_many(&mut self, count: u32) -> Result<bool> {
        for _ in 0..count {
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
    blanket_machine_method!(render_frame(self: &mut Self, palette: &Palette) -> image::RgbaImage);
    blanket_machine_method!(sound_active(self: &Self) -> bool);
    blanket_machine_method!(pitch(self: &Self) -> u8);
    blanket_machine_method!(audio_pattern(self: &Self) -> &[u8; 16]);
    blanket_machine_method!(memory(self: &Self) -> &[u8]);
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
    dynamic_machine_method!(render_frame(self: &mut Self, palette: &Palette) -> image::RgbaImage);
    dynamic_machine_method!(sound_active(self: &Self) -> bool);
    dynamic_machine_method!(pitch(self: &Self) -> u8);
    dynamic_machine_method!(audio_pattern(self: &Self) -> &[u8; 16]);
    dynamic_machine_method!(memory(self: &Self) -> &[u8]);
    dynamic_machine_method!(tick(self: &mut Self) -> Result<bool>);
    dynamic_machine_method!(tick_many(self: &mut Self, count: u32) -> Result<bool>);
}

struct Cpu {
    v: [u8; 16],
    i: u16,
    dt: u8,
    st: u8,
    pc: u16,
    sp: u4,
    stack: [u16; 16],
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
        self.v[u8::from(reg) as usize]
    }

    fn set_v(&mut self, reg: u4, val: u8) {
        self.v[u8::from(reg) as usize] = val;
    }

    fn push_stack(&mut self) -> Result<()> {
        if self.sp == u4::MAX {
            return Err(Error::StackFull);
        }
        self.sp += u4::new(1);
        self.stack[u8::from(self.sp) as usize] = self.pc;
        Ok(())
    }

    fn pop_stack(&mut self) -> Result<()> {
        if self.sp == u4::MIN {
            return Err(Error::StackEmpty);
        }
        self.pc = self.stack[u8::from(self.sp) as usize];
        self.sp -= u4::new(1);
        Ok(())
    }

    fn get_args(&self, args: Args) -> (u8, u8) {
        match args {
            Args::XKk { x, kk } => (self.get_v(x), kk),
            Args::XY { x, y } => (self.get_v(x), self.get_v(y)),
        }
    }

    fn get_args_mut(&mut self, args: Args) -> (&mut u8, u8) {
        match args {
            Args::XKk { x, kk } => (&mut self.v[u8::from(x) as usize], kk),
            Args::XY { x, y } => {
                let val = self.get_v(y);
                (&mut self.v[u8::from(x) as usize], val)
            }
        }
    }

    fn inc_pc(&mut self) -> Result<()> {
        self.pc = self
            .pc
            .checked_add(2)
            .ok_or(Error::PcOverflow(self.pc as u32 + 2))?;
        Ok(())
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

    pub fn render_frame(&mut self, palette: &Palette) -> image::RgbaImage {
        if self.cpu.dt > 0 {
            self.cpu.dt -= 1;
        }
        if self.cpu.st > 0 {
            self.cpu.st -= 1;
        }
        self.vblank = true;
        self.screen.to_image(palette)
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

    fn mem_slice<R>(&self, range: R) -> Result<&[u8]>
    where
        R: SliceIndex<[u8], Output = [u8]> + RangeBounds<usize> + Clone,
    {
        self.memory.get(range.clone()).ok_or_else(|| {
            Error::InvalidMemoryRange(
                (range.start_bound().cloned(), range.end_bound().cloned()),
                self.memory.len(),
            )
        })
    }

    fn mem_slice_mut<R>(&mut self, range: R) -> Result<&mut [u8]>
    where
        R: SliceIndex<[u8], Output = [u8]> + RangeBounds<usize> + Clone,
    {
        let memory_len = self.memory.len();
        self.memory.get_mut(range.clone()).ok_or_else(|| {
            Error::InvalidMemoryRange(
                (range.start_bound().cloned(), range.end_bound().cloned()),
                memory_len,
            )
        })
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
                self.cpu.inc_pc()?;
            }
            self.cpu.inc_pc()?;
        }
        Ok(())
    }

    fn read_word(&self) -> Result<u16> {
        let data = self.mem_slice(self.cpu.pc as usize..self.cpu.pc as usize + 2)?;
        Ok(u16::from_be_bytes(data.try_into().unwrap()))
    }

    // Returns a boolean specifying whether to exit
    pub fn tick(&mut self) -> Result<bool> {
        use Instruction as I;
        use SuperChipInstruction as Sci;
        use XoChipInstruction as Xci;

        // let span = info_span!("Chip8::tick", name = "Chip8::tick").entered();

        let word = self.read_word()?;
        // puffin::profile_function!(format!("{word:#06X}"));

        let raw_instruction = word;
        let instruction = Instruction::from_u16(raw_instruction, self.model.instruction_set())
            .ok_or(Error::InvalidInstruction(raw_instruction))?;
        self.cpu.inc_pc()?;

        // println!("Instruction: {raw_instruction:#06X}, {instruction:?}");
        match instruction {
            I::Exit0 => {
                if self.model.quirks().graceful_exit_on_0000 {
                    return Ok(true);
                } else {
                    return Err(Error::InvalidInstruction(raw_instruction));
                }
            }
            I::Cls => self.screen.clear(),
            I::Ret => self.cpu.pop_stack()?,
            I::Jp { nnn } => self.cpu.pc = nnn.into(),
            I::JpV0 { nnn } => {
                let reg = if self.model.quirks().jump_v0_use_vx {
                    (u16::from(nnn) >> 8) as usize
                } else {
                    0
                };
                self.cpu.pc = u16::from(nnn) + self.cpu.v[reg] as u16;
            }
            I::Call { nnn } => {
                self.cpu.push_stack()?;
                self.cpu.pc = nnn.into();
            }
            I::Se(args) => {
                let (a, b) = self.cpu.get_args(args);
                self.skip_if(a == b)?;
            }
            I::Sne(args) => {
                let (a, b) = self.cpu.get_args(args);
                self.skip_if(a != b)?;
            }
            I::Skp { x } => self.skip_if(self.keypad.is_pressed(self.cpu.get_v(x)))?,
            I::Sknp { x } => self.skip_if(!self.keypad.is_pressed(self.cpu.get_v(x)))?,
            I::Ld(args) => {
                let (reg, val) = self.cpu.get_args_mut(args);
                *reg = val;
            }
            I::LdI { nnn } => self.cpu.i = u16::from(nnn),
            I::LdToDt { x } => self.cpu.dt = self.cpu.get_v(x),
            I::LdFromDt { x } => self.cpu.set_v(x, self.cpu.dt),
            I::LdSt { x } => self.cpu.st = self.cpu.get_v(x),
            I::LdFromKey { x } => {
                if let Some(key) = self.keypad.test_event() {
                    self.cpu.set_v(x, u8::from(key));
                } else {
                    self.cpu.dec_pc();
                }
            }
            I::LdF { x } => {
                self.cpu.i = ((self.cpu.get_v(x) & 0xF) * screen::FONT[0].len() as u8) as u16
                    + screen::FONT_ADDRESS as u16
            }
            I::LdB { x } => {
                let digits = bcd(self.cpu.get_v(x));
                self.mem_slice_mut(self.cpu.i as usize..=self.cpu.i as usize + 2)?
                    .copy_from_slice(&digits);
            }
            I::LdToSlice { x } => {
                let mut data = [0; 16];
                let slice = &mut data[..=u8::from(x) as usize];
                slice.copy_from_slice(&self.cpu.v[..slice.len()]);
                self.mem_slice_mut(self.cpu.i as usize..self.cpu.i as usize + slice.len())?
                    .copy_from_slice(slice);
                if self.model.quirks().inc_i_on_slice {
                    self.cpu.i += slice.len() as u16;
                }
            }
            I::LdFromSlice { x } => {
                let mut data = [0; 16];
                let slice = &mut data[..=u8::from(x) as usize];
                slice.copy_from_slice(
                    self.mem_slice(self.cpu.i as usize..self.cpu.i as usize + slice.len())?,
                );
                self.cpu.v[..slice.len()].copy_from_slice(slice);
                if self.model.quirks().inc_i_on_slice {
                    self.cpu.i += slice.len() as u16;
                }
            }
            I::Add(Args::XKk { x, kk }) => {
                self.cpu.set_v(x, self.cpu.get_v(x).wrapping_add(kk));
            }
            I::Add(Args::XY { x, y }) => self.cpu.arithmetic_op_vf(x, y, u8::overflowing_add),
            I::AddI { x } => self.cpu.i = self.cpu.i.wrapping_add(self.cpu.get_v(x) as u16),
            I::Or { x, y } => self.cpu.arithmetic_op(
                x,
                y,
                std::ops::BitOr::bitor,
                self.model.quirks().bitwise_reset_flag,
            ),
            I::And { x, y } => self.cpu.arithmetic_op(
                x,
                y,
                std::ops::BitAnd::bitand,
                self.model.quirks().bitwise_reset_flag,
            ),
            I::Xor { x, y } => self.cpu.arithmetic_op(
                x,
                y,
                std::ops::BitXor::bitxor,
                self.model.quirks().bitwise_reset_flag,
            ),
            I::Shl { x, y } => self.cpu.arithmetic_op_vf(
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
            ),
            I::Shr { x, y } => self.cpu.arithmetic_op_vf(
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
            ),
            I::Sub { x, y } => self.cpu.arithmetic_op_vf(x, y, |a, b| {
                let (result, borrow) = a.overflowing_sub(b);
                (result, !borrow)
            }),
            I::Subn { x, y } => self.cpu.arithmetic_op_vf(x, y, |a, b| {
                let (result, borrow) = b.overflowing_sub(a);
                (result, !borrow)
            }),
            I::Rnd { x, kk } => self.cpu.set_v(x, self.rng.random::<u8>() & kk),
            I::Drw { x, y, n } => {
                if self.draw_wait_for_vblank() && !self.vblank {
                    self.cpu.dec_pc();
                } else {
                    let x_val = self.cpu.get_v(x);
                    let y_val = self.cpu.get_v(y);
                    let mut data = [0; 64];
                    let slice = &mut data[..u8::from(n) as usize * self.screen.num_active_planes()];
                    slice.copy_from_slice(
                        self.mem_slice(self.cpu.i as usize..self.cpu.i as usize + slice.len())?,
                    );
                    self.cpu.v[0xF] = self.screen.draw_sprite(x_val, y_val, slice) as u8;
                }
            }
            I::SuperChip(sc_instruction) => {
                if self.model.instruction_set() >= InstructionSet::SuperChip {
                    match sc_instruction {
                        Sci::Exit => return Ok(true),
                        Sci::LoRes => {
                            self.screen.set_hires(false)?;
                            if self.model.quirks().clear_screen_on_mode_switch {
                                self.screen.clear();
                            }
                        }
                        Sci::HiRes => {
                            self.screen.set_hires(true)?;
                            if self.model.quirks().clear_screen_on_mode_switch {
                                self.screen.clear();
                            }
                        }
                        Sci::DrawLarge { x, y } => {
                            if self.draw_wait_for_vblank() && !self.vblank {
                                self.cpu.dec_pc();
                            } else {
                                let x_val = self.cpu.get_v(x);
                                let y_val = self.cpu.get_v(y);
                                if self.model.quirks().lores_draw_large_as_small
                                    && !self.screen.get_hires()
                                {
                                    let mut data = [0; 64];
                                    let slice = &mut data[..16 * self.screen.num_active_planes()];
                                    slice.copy_from_slice(self.mem_slice(
                                        self.cpu.i as usize..self.cpu.i as usize + slice.len(),
                                    )?);
                                    self.cpu.v[0xF] =
                                        self.screen.draw_sprite(x_val, y_val, slice) as u8;
                                } else {
                                    let mut data = [0; 128];
                                    let slice = &mut data[..32 * self.screen.num_active_planes()];
                                    slice.copy_from_slice(self.mem_slice(
                                        self.cpu.i as usize..self.cpu.i as usize + slice.len(),
                                    )?);
                                    self.cpu.v[0xF] = self.screen.draw_large_sprite(
                                        x_val,
                                        y_val,
                                        bytemuck::cast_slice(slice),
                                    )?;
                                }
                            }
                        }
                        Sci::StoreRegs { x } => self.rpl[..=u8::from(x) as usize]
                            .copy_from_slice(&self.cpu.v[..=u8::from(x) as usize]),
                        Sci::GetRegs { x } => self.cpu.v[..=u8::from(x) as usize]
                            .copy_from_slice(&self.rpl[..=u8::from(x) as usize]),
                        Sci::ScrollDown { n } => self.screen.scroll_down(n)?,
                        Sci::ScrollRight => self.screen.scroll_right()?,
                        Sci::ScrollLeft => self.screen.scroll_left()?,
                        Sci::LdHiResF { x } => {
                            self.cpu.i = ((self.cpu.get_v(x) & 0xF)
                                * screen::XOCHIP_HIRES_FONT[0].len() as u8)
                                as u16
                                + screen::XOCHIP_HIRES_FONT_ADDRESS as u16
                        }
                    }
                } else {
                    self.cpu.dec_pc();
                    return Err(Error::UnsupportedInstruction(instruction));
                }
            }
            I::XoChip(xc_instruction) => {
                if self.model.instruction_set() >= InstructionSet::XoChip {
                    match xc_instruction {
                        Xci::ScrollUp { n } => self.screen.scroll_up(n)?,
                        Xci::RegRangeToMem { x, y } => {
                            let x = u8::from(x) as usize;
                            let y = u8::from(y) as usize;
                            let mut data = [0; 16];
                            let slice = &mut data[..=x.abs_diff(y)];
                            if y >= x {
                                slice.copy_from_slice(&self.cpu.v[x..=y]);
                            } else {
                                slice.copy_from_slice(&self.cpu.v[y..=x]);
                                slice.reverse();
                            }
                            self.mem_slice_mut(
                                self.cpu.i as usize..self.cpu.i as usize + slice.len(),
                            )?
                            .copy_from_slice(slice);
                        }
                        Xci::RegRangeFromMem { x, y } => {
                            let x = u8::from(x) as usize;
                            let y = u8::from(y) as usize;
                            let mut data = [0; 16];
                            let slice = &mut data[..=x.abs_diff(y)];
                            slice.copy_from_slice(self.mem_slice(
                                self.cpu.i as usize..self.cpu.i as usize + slice.len(),
                            )?);
                            if y >= x {
                                self.cpu.v[x..=y].copy_from_slice(slice);
                            } else {
                                slice.reverse();
                                self.cpu.v[y..=x].copy_from_slice(slice);
                            }
                        }
                        Xci::LdLong => {
                            let addr = self.read_word()?;
                            self.cpu.inc_pc()?;
                            self.cpu.i = addr;
                        }
                        Xci::SelectPlanes { x } => self.screen.set_planes(x)?,
                        Xci::WriteAudio => {
                            let mut data = [0; 16];
                            data.copy_from_slice(
                                self.mem_slice(self.cpu.i as usize..self.cpu.i as usize + 16)?,
                            );
                            self.audio_pattern = data;
                        }
                        Xci::SetPitch { x } => self.pitch = self.cpu.get_v(x),
                    }
                } else {
                    self.cpu.dec_pc();
                    return Err(Error::UnsupportedInstruction(instruction));
                }
            }
        }

        if self.vblank {
            self.vblank = false;
        }

        Ok(false)
    }
}

fn bcd(x: u8) -> [u8; 3] {
    [x / 100, x / 10 % 10, x % 10]
}
