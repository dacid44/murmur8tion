use std::ops::{Add, Bound, RangeBounds, Sub};

use rand::Rng;
use thiserror::Error;
use ux::{u12, u4};

use crate::{
    instruction::{Args, Instruction, InstructionSet, SuperChipInstruction},
    model::{self, CosmacVip, DynamicModel, LegacySuperChip, ModernSuperChip},
    screen::{self, Palette, Screen},
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid instruction '{0:#06X}'")]
    InvalidInstruction(u16),
    #[error("the program counter tried to add past available memory (address {0:#06X})")]
    PcOverflow(u16),
    #[error("ret was called with no return value on the stack")]
    StackEmpty,
    #[error("call was called when the stack was full")]
    StackFull,
    #[error("an invalid memory address was accessed (address {0:#06X})")]
    InvalidAddress(usize),
    #[error("an unsupported screen operation was run")]
    UnsupportedScreenOperation(#[from] screen::UnsupportedScreenOperation),
}

type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! dynamic_machine_method {
    ($name:ident(self: $($selfty:ty)?$(, $param:ident: $ptype:ty)*)$( -> $ret:ty)?) => {
        pub fn $name(self$(: $selfty)?$(, $param: $ptype)*)$( -> $ret)? {
            match self {
                Self::CosmacVip(machine) => Chip8::$name(machine$(, $param)*),
                Self::LegacySuperChip(machine) => Chip8::$name(machine$(, $param)*),
                Self::ModernSuperChip(machine) => Chip8::$name(machine$(, $param)*),
            }
        }
    }
}

pub enum DynamicMachine {
    CosmacVip(Chip8<CosmacVip>),
    LegacySuperChip(Chip8<LegacySuperChip>),
    ModernSuperChip(Chip8<ModernSuperChip>),
}

impl DynamicMachine {
    pub fn new(model: &DynamicModel, rom: &[u8]) -> Self {
        match model {
            DynamicModel::CosmacVip => Self::new_cosmac_vip(rom),
            DynamicModel::LegacySuperChip => Self::new_legacy_schip(rom),
            DynamicModel::ModernSuperChip => Self::new_modern_schip(rom),
        }
    }

    pub fn new_cosmac_vip(rom: &[u8]) -> Self {
        Self::CosmacVip(Chip8::new(CosmacVip, rom))
    }

    pub fn new_legacy_schip(rom: &[u8]) -> Self {
        Self::LegacySuperChip(Chip8::new(LegacySuperChip, rom))
    }

    pub fn new_modern_schip(rom: &[u8]) -> Self {
        Self::ModernSuperChip(Chip8::new(ModernSuperChip, rom))
    }

    dynamic_machine_method!(event(self: &mut Self, key: u4, event: KeyEvent));
    dynamic_machine_method!(render_frame(self: &mut Self, palette: &Palette) -> image::RgbaImage);
    dynamic_machine_method!(sound_active(self: &Self) -> bool);
    dynamic_machine_method!(tick(self: &mut Self) -> Result<bool>);
}

struct Cpu {
    v: [u8; 16],
    i: u16,
    dt: u8,
    st: u8,
    pc: u12,
    sp: u4,
    stack: [u12; 16],
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            v: [0; 16],
            i: 0,
            dt: 0,
            st: 0,
            pc: u12::new(0x200u16),
            sp: u4::MIN,
            stack: [u12::MIN; 16],
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
        self.sp = self.sp + u4::new(1);
        self.stack[u8::from(self.sp) as usize] = self.pc;
        Ok(())
    }

    fn pop_stack(&mut self) -> Result<()> {
        if self.sp == u4::MIN {
            return Err(Error::StackEmpty);
        }
        self.pc = self.stack[u8::from(self.sp) as usize];
        self.sp = self.sp - u4::new(1);
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

    fn try_set_pc(&mut self, pc: u16) -> Result<()> {
        self.pc = pc.try_into().map_err(|_| Error::PcOverflow(pc))?;
        Ok(())
    }

    fn inc_pc(&mut self) -> Result<()> {
        self.try_set_pc(u16::from(self.pc) + 2)
    }

    fn dec_pc(&mut self) {
        self.pc = self.pc - u12::new(2);
    }

    fn skip_if(&mut self, condition: bool) -> Result<()> {
        if condition {
            self.inc_pc()?;
        }
        Ok(())
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
    event: Option<(u4, KeyEvent)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Press,
    Release,
}

impl Keypad {
    fn event(&mut self, key: u4, event: KeyEvent) {
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
        if record_event {
            self.event = Some((key, event));
        }
    }

    fn test_event(&mut self, event: KeyEvent) -> Option<u4> {
        match (self.waiting, self.event) {
            (true, Some((key, kind))) if kind == event => {
                self.waiting = false;
                Some(key)
            }
            (true, _) => None,
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

pub struct Chip8<Model: model::Model> {
    model: Model,
    keypad: Keypad,
    cpu: Cpu,
    memory: Box<[u8; 0x1000]>,
    screen: Model::Screen,
    rng: Model::Rng,
    vblank: bool,
    rpl: [u8; 16],
}

impl<Model: model::Model> Chip8<Model> {
    pub fn new(model: Model, rom: &[u8]) -> Self {
        let mut memory = Box::new([0; 0x1000]);
        let font_slice: &[u8] = screen::FONT.as_flattened();
        memory[screen::FONT_ADDRESS..screen::FONT_ADDRESS + font_slice.len()]
            .copy_from_slice(font_slice);
        let hires_font_slice: &[u8] = screen::SCHIP_HIRES_FONT.as_flattened();
        memory[screen::SCHIP_HIRES_FONT_ADDRESS
            ..screen::SCHIP_HIRES_FONT_ADDRESS + hires_font_slice.len()]
            .copy_from_slice(hires_font_slice);
        memory[0x200..0x200 + rom.len()].copy_from_slice(rom);
        let screen = model.init_screen();
        let rng = model.init_rng();
        Self {
            keypad: Default::default(),
            model,
            cpu: Default::default(),
            memory,
            screen,
            rng,
            vblank: false,
            rpl: [0; 16],
        }
    }

    pub fn event(&mut self, key: u4, event: KeyEvent) {
        self.keypad.event(key, event)
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

    fn mem_slice<R: MemBounds<I>, I>(&self, range: R) -> Result<&[u8]> {
        Ok(&self.memory[u16::from(range.start_inclusive()?) as usize
            ..=u16::from(range.end_inclusive()?) as usize])
    }

    fn mem_slice_mut<R: MemBounds<I>, I>(&mut self, range: R) -> Result<&mut [u8]> {
        Ok(
            &mut self.memory[u16::from(range.start_inclusive()?) as usize
                ..=u16::from(range.end_inclusive()?) as usize],
        )
    }

    fn draw_wait_for_vblank(&self) -> bool {
        self.model
            .quirks()
            .draw_wait_for_vblank
            .wait(self.screen.get_hires())
    }

    // Returns a boolean specifying whether to exit
    pub fn tick(&mut self) -> Result<bool> {
        use Instruction as I;
        use SuperChipInstruction as Sci;

        let raw_instruction = u16::from_be_bytes([
            self.memory[u16::from(self.cpu.pc) as usize],
            self.memory[u16::from(self.cpu.pc) as usize + 1],
        ]);
        let instruction = Instruction::from_u16(raw_instruction, self.model.instruction_set())
            .ok_or(Error::InvalidInstruction(raw_instruction))?;
        self.cpu.inc_pc()?;

        // println!("Instruction: {raw_instruction:#06X}, {instruction:?}");
        match instruction {
            I::Sys { .. } => {}
            I::Cls => self.screen.clear(),
            I::Ret => self.cpu.pop_stack()?,
            I::Jp { nnn } => self.cpu.pc = nnn,
            I::JpV0 { nnn } => {
                let reg = if self.model.quirks().jump_v0_use_vx {
                    (u16::from(nnn) >> 8) as usize
                } else {
                    0
                };
                self.cpu
                    .try_set_pc(u16::from(nnn) + self.cpu.v[reg] as u16)?
            }
            I::Call { nnn } => {
                self.cpu.push_stack()?;
                self.cpu.pc = nnn;
            }
            I::Se(args) => {
                let (a, b) = self.cpu.get_args(args);
                self.cpu.skip_if(a == b)?;
            }
            I::Sne(args) => {
                let (a, b) = self.cpu.get_args(args);
                self.cpu.skip_if(a != b)?;
            }
            I::Skp { x } => self
                .cpu
                .skip_if(self.keypad.is_pressed(self.cpu.get_v(x)))?,
            I::Sknp { x } => self
                .cpu
                .skip_if(!self.keypad.is_pressed(self.cpu.get_v(x)))?,
            I::Ld(args) => {
                let (reg, val) = self.cpu.get_args_mut(args);
                *reg = val;
            }
            I::LdI { nnn } => self.cpu.i = u16::from(nnn),
            I::LdToDt { x } => self.cpu.dt = self.cpu.get_v(x),
            I::LdFromDt { x } => self.cpu.set_v(x, self.cpu.dt),
            I::LdSt { x } => self.cpu.st = self.cpu.get_v(x),
            I::LdFromKey { x } => {
                if let Some(key) = self.keypad.test_event(self.model.quirks().key_wait_trigger) {
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
                self.mem_slice_mut(self.cpu.i..=self.cpu.i + 2)?
                    .copy_from_slice(&digits);
            }
            I::LdToSlice { x } => {
                let mut data = [0; 16];
                let slice = &mut data[..=u8::from(x) as usize];
                slice.copy_from_slice(&self.cpu.v[..slice.len()]);
                self.mem_slice_mut(self.cpu.i..self.cpu.i + slice.len() as u16)?
                    .copy_from_slice(slice);
                if self.model.quirks().inc_i_on_slice {
                    self.cpu.i += slice.len() as u16;
                }
            }
            I::LdFromSlice { x } => {
                let mut data = [0; 16];
                let slice = &mut data[..=u8::from(x) as usize];
                slice.copy_from_slice(self.mem_slice(self.cpu.i..self.cpu.i + slice.len() as u16)?);
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
                    let mut data = [0; 16];
                    let slice = &mut data[..u8::from(n) as usize];
                    slice.copy_from_slice(self.mem_slice(self.cpu.i..self.cpu.i + u16::from(n))?);
                    self.cpu.v[0xF] = self.screen.draw_sprite(x_val, y_val, slice) as u8;
                }
            }
            I::SuperChip(instruction) => {
                if self.model.instruction_set() >= InstructionSet::SuperChip {
                    match instruction {
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
                                    let mut data = [0; 16];
                                    data.copy_from_slice(
                                        self.mem_slice(self.cpu.i..self.cpu.i + 16)?,
                                    );
                                    self.cpu.v[0xF] =
                                        self.screen.draw_sprite(x_val, y_val, &data) as u8;
                                } else {
                                    let mut data = [0; 32];
                                    data.copy_from_slice(
                                        self.mem_slice(self.cpu.i..self.cpu.i + 32)?,
                                    );
                                    self.cpu.v[0xF] =
                                        self.screen.draw_large_sprite(x_val, y_val, &data)?;
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
                                * screen::SCHIP_HIRES_FONT[0].len() as u8)
                                as u16
                                + screen::SCHIP_HIRES_FONT_ADDRESS as u16
                        }
                    }
                } else {
                    self.cpu.dec_pc();
                    return Err(Error::InvalidInstruction(raw_instruction));
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

trait MemBounds<I> {
    fn start_inclusive(&self) -> Result<u12>;
    fn end_inclusive(&self) -> Result<u12>;
}

impl<R, I> MemBounds<I> for R
where
    R: RangeBounds<I>,
    I: Copy + TryInto<u12> + Into<usize> + From<u8> + Add<I, Output = I> + Sub<I, Output = I>,
{
    fn start_inclusive(&self) -> Result<u12> {
        match self.start_bound() {
            Bound::Included(&start) => start
                .try_into()
                .map_err(|_| Error::InvalidAddress(start.into())),
            Bound::Excluded(&start) => {
                let start_inclusive = start + I::from(1);
                start_inclusive
                    .try_into()
                    .map_err(|_| Error::InvalidAddress(start_inclusive.into()))
            }
            Bound::Unbounded => Ok(u12::MIN),
        }
    }

    fn end_inclusive(&self) -> Result<u12> {
        match self.end_bound() {
            Bound::Included(&end) => end
                .try_into()
                .map_err(|_| Error::InvalidAddress(end.into())),
            Bound::Excluded(&end) => {
                let end_inclusive = end - I::from(1);
                end_inclusive
                    .try_into()
                    .map_err(|_| Error::InvalidAddress(end_inclusive.into()))
            }
            Bound::Unbounded => Ok(u12::MAX),
        }
    }
}
