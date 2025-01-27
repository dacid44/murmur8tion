use ux::{u12, u2, u4};

macro_rules! match_ux {
    ($type:path; $match_type:ident; $max:literal; $x: expr; $name: ident; $($match:tt)*) => {
        {
            let $name: $type = $x;
            match $match_type::from($name) {
                $max.. => unreachable!(),
                $($match)*
            }
        }
    };
}

macro_rules! match_u4 {
    ($x:expr; $name: ident; $($match:tt)*) => { match_ux! { ::ux::u4; u8; 0x10; $x; $name; $($match)* } };
    ($x:expr; $($match:tt)*) => { match_u4!{$x; x; $($match)* } };
}

macro_rules! match_u12 {
    ($x:expr; $name: ident; $($match:tt)*) => { match_ux! { ::ux::u12; u16; 0x1000; $x; $name; $($match)* } };
    ($x:expr; $($match:tt)*) => { match_u12!{$x; x; $($match)* } };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawInstruction(u16);

impl RawInstruction {
    fn nibble(self, i: u2) -> u4 {
        (self.0 >> (u8::from(i) * 4) & 0x00F0).try_into().unwrap()
    }

    fn discriminant1(self) -> u4 {
        self.nibble(u2::new(3))
    }

    fn discriminant2(self) -> u8 {
        (self.0 & 0x00FF).try_into().unwrap()
    }

    fn discriminant3(self) -> u4 {
        self.nibble(u2::new(0))
    }

    fn nnn(self) -> u12 {
        (self.0 & 0x0FFF).try_into().unwrap()
    }

    fn n(self) -> u4 {
        self.nibble(u2::new(0))
    }

    fn x(self) -> u4 {
        self.nibble(u2::new(2))
    }

    fn y(self) -> u4 {
        self.nibble(u2::new(1))
    }

    fn kk(self) -> u8 {
        (self.0 & 0x00FF).try_into().unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Args {
    XKk { x: u4, kk: u8 },
    XY { x: u4, y: u4 },
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Sys { nnn: u12 },
    Cls,
    Ret,
    Jp { nnn: u12 },
    JpV0 { nnn: u12 },
    Call { nnn: u12 },
    Se(Args),
    Sne(Args),
    Skp { x: u4 },
    Sknp { x: u4 },
    Ld(Args),
    LdI { nnn: u12 },
    LdToDt { x: u4 },
    LdFromDt { x: u4 },
    LdSt { x: u4 },
    LdFromKey { x: u4 },
    LdF { x: u4 },
    LdB { x: u4 },
    LdToSlice { x: u4 },
    LdFromSlice { x: u4 },
    Add(Args),
    AddI { x: u4 },
    Or { x: u4, y: u4 },
    And { x: u4, y: u4 },
    Xor { x: u4, y: u4 },
    Shl { x: u4, y: u4 },
    Shr { x: u4, y: u4 },
    Sub { x: u4, y: u4 },
    Subn { x: u4, y: u4 },
    Rnd { x: u4, kk: u8 },
    Drw { x: u4, y: u4, n: u4 },
    // Super Chip-48 instructions after
}

impl Instruction {
    pub fn from_u16(instruction: u16) -> Option<Self> {
        let instruction = RawInstruction(instruction);
        Some(match_u4! {instruction.discriminant1();
            0x0 => match_u12! {instruction.nnn(); nnn;
                0x0E0 => Self::Cls,
                0x0EE => Self::Ret,
                _ => Self::Sys { nnn },
            },
            0x1 => Self::Jp { nnn: instruction.nnn() },
            0x2 => Self::Call { nnn: instruction.nnn() },
            0x3 => Self::Se(Args::XKk { x: instruction.x(), kk: instruction.kk() }),
            0x4 => Self::Sne(Args::XKk { x: instruction.x(), kk: instruction.kk() }),
            0x5 if instruction.discriminant3() == u4::new(0x0) => Self::Se(Args::XY { x: instruction.x(), y: instruction.y() }),
            0x6 => Self::Ld(Args::XKk { x: instruction.x(), kk: instruction.kk() }),
            0x7 => Self::Add(Args::XKk { x: instruction.x(), kk: instruction.kk() }),
            0x8 => match_u4! {instruction.discriminant3();
                0x0 => Self::Ld(Args::XY { x: instruction.x(), y: instruction.y() }),
                0x1 => Self::Or { x: instruction.x(), y: instruction.y() },
                0x2 => Self::And { x: instruction.x(), y: instruction.y() },
                0x3 => Self::Xor { x: instruction.x(), y: instruction.y() },
                0x4 => Self::Add(Args::XY { x: instruction.x(), y: instruction.y() }),
                0x5 => Self::Sub { x: instruction.x(), y: instruction.y() },
                0x6 => Self::Shr { x: instruction.x(), y: instruction.y() },
                0x7 => Self::Subn { x: instruction.x(), y: instruction.y() },
                0xE => Self::Shl { x: instruction.x(), y: instruction.y() },
                _ => return None,
            },
            0x9 if instruction.discriminant3() == u4::new(0x0) => Self::Sne(Args::XY { x: instruction.x(), y: instruction.y() }),
            0xA => Self::LdI { nnn: instruction.nnn() },
            0xB => Self::JpV0 { nnn: instruction.nnn() },
            0xC => Self::Rnd { x: instruction.x(), kk: instruction.kk() },
            0xD => Self::Drw { x: instruction.x(), y: instruction.y(), n: instruction.n() },
            0xE => match instruction.discriminant2() {
                0x9E => Self::Skp { x: instruction.x() },
                0xA1 => Self::Sknp { x: instruction.x() },
                _ => return None,
            },
            0xF => match instruction.discriminant2() {
                0x07 => Self::LdFromDt { x: instruction.x() },
                0x0A => Self::LdFromKey { x: instruction.x() },
                0x15 => Self::LdToDt { x: instruction.x() },
                0x18 => Self::LdSt { x: instruction.x() },
                0x1E => Self::AddI { x: instruction.x() },
                0x29 => Self::LdF { x: instruction.x() },
                0x33 => Self::LdB { x: instruction.x() },
                0x55 => Self::LdToSlice { x: instruction.x() },
                0x65 => Self::LdFromSlice { x: instruction.x() },
                _ => return None,
            }
            _ => return None,
        })
    }
}
