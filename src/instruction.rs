use arbitrary_int::{u12, u4};
use bitbybit::bitfield;
use tracing::info_span;

macro_rules! match_ux {
    ($type:path; $match_type:ident; $max:literal; $x: expr; $name: ident; $($match:tt)*) => {
        {
            let $name: $type = $x;
            const UNREACHABLE_START: $match_type = ::arbitrary_int::UInt::value(<$type as ::arbitrary_int::Number>::MAX) + 1;
            match $match_type::from($name) {
                // UNREACHABLE_START.. => unsafe { ::std::hint::unreachable_unchecked() },
                UNREACHABLE_START => unreachable!(),
                $($match)*
            }
        }
    };
}

macro_rules! match_u4 {
    ($x:expr; $name: ident; $($match:tt)*) => { match_ux! { ::arbitrary_int::u4; u8; 0x10; $x; $name; $($match)* } };
    ($x:expr; $($match:tt)*) => { match_u4!{$x; x; $($match)* } };
}

macro_rules! match_u12 {
    ($x:expr; $name: ident; $($match:tt)*) => { match_ux! { ::arbitrary_int::u12; u16; 0x1000; $x; $name; $($match)* } };
    ($x:expr; $($match:tt)*) => { match_u12!{$x; x; $($match)* } };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstructionSet {
    CosmacVip,
    SuperChip,
    XoChip,
}

#[bitfield(u16)]
struct RawInstruction {
    #[bits(0..=3, r)]
    nibble: [u4; 4],

    #[bits(12..=15, r)]
    discriminant1: u4,

    #[bits(0..=7, r)]
    discriminant2: u8,

    #[bits(0..=3, r)]
    discriminant3: u4,

    #[bits(0..=11, r)]
    nnn: u12,

    #[bits(0..=3, r)]
    n: u4,

    #[bits(8..=11, r)]
    x: u4,

    #[bits(4..=7, r)]
    y: u4,

    #[bits(0..=7, r)]
    kk: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum Args {
    XKk { x: u4, kk: u8 },
    XY { x: u4, y: u4 },
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Exit0,
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
    SuperChip(SuperChipInstruction),
    XoChip(XoChipInstruction),
}

#[derive(Debug, Clone, Copy)]
pub enum SuperChipInstruction {
    Exit,
    LoRes,
    HiRes,
    DrawLarge { x: u4, y: u4 },
    StoreRegs { x: u4 },
    GetRegs { x: u4 },
    ScrollDown { n: u4 },
    ScrollRight,
    ScrollLeft,
    LdHiResF { x: u4 },
}

#[derive(Debug, Clone, Copy)]
pub enum XoChipInstruction {
    ScrollUp { n: u4 },
    RegRangeToMem { x: u4, y: u4 },
    RegRangeFromMem { x: u4, y: u4 },
    LdLong,
    SelectPlanes { x: u4 },
    WriteAudio,
    SetPitch { x: u4 },
}

impl Instruction {
    #[inline(always)]
    pub fn from_u16(instruction: u16, instruction_set: InstructionSet) -> Option<Self> {
        use Instruction::SuperChip as Sc;
        use Instruction::XoChip as Xc;
        use SuperChipInstruction as Sci;
        use XoChipInstruction as Xci;
        // let span = info_span!("Instruction::from_u16", name = "Instruction::from_u16").entered();

        let instruction = RawInstruction::new_with_raw_value(instruction);
        Some(match_u4! {instruction.discriminant1();
            0x0 => match_u12! {instruction.nnn(); nnn;
                0x000 => Self::Exit0,
                0x0E0 => Self::Cls,
                0x0EE => Self::Ret,
                _ if instruction_set >= InstructionSet::SuperChip => match_u12! {nnn;
                    0x0C0..=0x0CF => Sc(Sci::ScrollDown { n: instruction.n() }),
                    0x0D0..=0x0DF if instruction_set >= InstructionSet::XoChip => {
                        Xc(Xci::ScrollUp { n: instruction.n() })
                    }
                    0x0FB => Sc(Sci::ScrollRight),
                    0x0FC => Sc(Sci::ScrollLeft),
                    0x0FD => Sc(Sci::Exit),
                    0x0FE => Sc(Sci::LoRes),
                    0x0FF => Sc(Sci::HiRes),
                    _ => return None,
                },
                _ => return None,
            },
            0x1 => Self::Jp { nnn: instruction.nnn() },
            0x2 => Self::Call { nnn: instruction.nnn() },
            0x3 => Self::Se(Args::XKk { x: instruction.x(), kk: instruction.kk() }),
            0x4 => Self::Sne(Args::XKk { x: instruction.x(), kk: instruction.kk() }),
            0x5 if instruction.discriminant3() == u4::new(0x0) => Self::Se(Args::XY { x: instruction.x(), y: instruction.y() }),
            0x5 => match_u4! {instruction.discriminant3();
                0x0 => Self::Se(Args::XY { x: instruction.x(), y: instruction.y() }),
                0x2 if instruction_set >= InstructionSet::XoChip => Xc(Xci::RegRangeToMem { x: instruction.x(), y: instruction.y() }),
                0x3 if instruction_set >= InstructionSet::XoChip => Xc(Xci::RegRangeFromMem { x: instruction.x(), y: instruction.y() }),
                _ => return None,
            },
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
            0xD => if instruction_set >= InstructionSet::SuperChip && instruction.discriminant3() == u4::new(0x0) {
                Sc(Sci::DrawLarge { x: instruction.x(), y: instruction.y() })
            } else {
                Self::Drw { x: instruction.x(), y: instruction.y(), n: instruction.n() }
            },
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
                disc2 if instruction_set >= InstructionSet::SuperChip => match disc2 {
                    0x30 => Sc(Sci::LdHiResF { x: instruction.x() }),
                    0x75 => Sc(Sci::StoreRegs { x: instruction.x() }),
                    0x85 => Sc(Sci::GetRegs { x: instruction.x() }),
                    _ if instruction_set >= InstructionSet::XoChip => match disc2 {
                        0x00 if instruction.x() == u4::new(0) => Xc(Xci::LdLong),
                        0x01 => Xc(Xci::SelectPlanes { x: instruction.x() }),
                        0x02 if instruction.x() == u4::new(0) => Xc(Xci::WriteAudio),
                        0x3A => Xc(Xci::SetPitch { x: instruction.x() }),
                       _ => return None,
                    },
                    _ => return None,
                },
                _ => return None,
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod test {
    use arbitrary_int::{u12, u4};

    use super::RawInstruction;

    #[test]
    fn test_nibbles() {
        let raw_instruction = RawInstruction::new_with_raw_value(0x1234);
        assert_eq!(raw_instruction.nibble(0), u4::new(0x4));
        assert_eq!(raw_instruction.nibble(1), u4::new(0x3));
        assert_eq!(raw_instruction.nibble(2), u4::new(0x2));
        assert_eq!(raw_instruction.nibble(3), u4::new(0x1));

        assert_eq!(raw_instruction.discriminant1(), u4::new(0x1));
        assert_eq!(raw_instruction.discriminant2(), 0x34);
        assert_eq!(raw_instruction.discriminant3(), u4::new(0x4));
        assert_eq!(raw_instruction.nnn(), u12::new(0x234));
        assert_eq!(raw_instruction.n(), u4::new(0x4));
        assert_eq!(raw_instruction.x(), u4::new(0x2));
        assert_eq!(raw_instruction.y(), u4::new(0x3));
        assert_eq!(raw_instruction.kk(), 0x34);
    }
}
