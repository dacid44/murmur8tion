use arbitrary_int::{u12, u4};
use bitbybit::bitfield;

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
    LdILong,
    SelectPlanes { x: u4 },
    WriteAudio,
    SetPitch { x: u4 },
}

impl Instruction {
    #[inline(always)]
    pub fn from_u16(instruction: u16, instruction_set: InstructionSet) -> Option<Self> {
        use Instruction::SuperChip as Sc;
        use Instruction::XoChip as Xc;
        use InstructionSet::SuperChip as IsSc;
        use InstructionSet::XoChip as IsXc;
        use SuperChipInstruction as Sci;
        use XoChipInstruction as Xci;
        // let span = info_span!("Instruction::from_u16", name = "Instruction::from_u16").entered();

        // let instruction = RawInstruction::new_with_raw_value(instruction);
        // let disc1 = instruction.discriminant1();
        // let x = instruction.x();
        // let x_u8 = x.value();
        // let y = instruction.y();
        // let y_u8 = y.value();
        // let n = instruction.n();
        // let n_u8 = n.value();
        // let kk = instruction.kk();
        // let nnn = instruction.nnn();

        let [disc1, y_u8] = ((instruction & 0xF0F0) >> 4).to_be_bytes();
        let [x_u8, n_u8] = (instruction & 0x0F0F).to_be_bytes();
        let [x, y, n] = [x_u8, y_u8, n_u8].map(|nibble| unsafe { u4::new_unchecked(nibble) });
        let kk = (instruction & 0xFF) as u8;
        let nnn = u12::new(instruction & 0xFFF);
        Some(match (disc1, x_u8, y_u8, n_u8, instruction_set) {
            (0x10.., _, _, _, _)
            | (_, 0x10.., _, _, _)
            | (_, _, 0x10.., _, _)
            | (_, _, _, 0x10.., _) => unsafe { std::hint::unreachable_unchecked() },
            (0x0, 0x0, 0x0, 0x0, _) => Self::Exit0,
            (0x0, 0x0, 0xC, _, IsSc | IsXc) => Sc(Sci::ScrollDown { n }),
            (0x0, 0x0, 0xD, _, IsXc) => Xc(Xci::ScrollUp { n }),
            (0x0, 0x0, 0xE, 0x0, _) => Self::Cls,
            (0x0, 0x0, 0xE, 0xE, _) => Self::Ret,
            (0x0, 0x0, 0xF, 0xB, IsSc | IsXc) => Sc(Sci::ScrollRight),
            (0x0, 0x0, 0xF, 0xC, IsSc | IsXc) => Sc(Sci::ScrollLeft),
            (0x0, 0x0, 0xF, 0xD, IsSc | IsXc) => Sc(Sci::Exit),
            (0x0, 0x0, 0xF, 0xE, IsSc | IsXc) => Sc(Sci::LoRes),
            (0x0, 0x0, 0xF, 0xF, IsSc | IsXc) => Sc(Sci::HiRes),
            (0x1, _, _, _, _) => Self::Jp { nnn },
            (0x2, _, _, _, _) => Self::Call { nnn },
            (0x3, _, _, _, _) => Self::Se(Args::XKk { x, kk }),
            (0x4, _, _, _, _) => Self::Sne(Args::XKk { x, kk }),
            (0x5, _, _, 0x0, _) => Self::Se(Args::XY { x, y }),
            (0x5, _, _, 0x2, IsXc) => Xc(Xci::RegRangeToMem { x, y }),
            (0x5, _, _, 0x3, IsXc) => Xc(Xci::RegRangeFromMem { x, y }),
            (0x6, _, _, _, _) => Self::Ld(Args::XKk { x, kk }),
            (0x7, _, _, _, _) => Self::Add(Args::XKk { x, kk }),
            (0x8, _, _, 0x0, _) => Self::Ld(Args::XY { x, y }),
            (0x8, _, _, 0x1, _) => Self::Or { x, y },
            (0x8, _, _, 0x2, _) => Self::And { x, y },
            (0x8, _, _, 0x3, _) => Self::Xor { x, y },
            (0x8, _, _, 0x4, _) => Self::Add(Args::XY { x, y }),
            (0x8, _, _, 0x5, _) => Self::Sub { x, y },
            (0x8, _, _, 0x6, _) => Self::Shr { x, y },
            (0x8, _, _, 0x7, _) => Self::Subn { x, y },
            (0x8, _, _, 0xE, _) => Self::Shl { x, y },
            (0x9, _, _, 0x0, _) => Self::Sne(Args::XY { x, y }),
            (0xA, _, _, _, _) => Self::LdI { nnn },
            (0xB, _, _, _, _) => Self::JpV0 { nnn },
            (0xC, _, _, _, _) => Self::Rnd { x, kk },
            (0xD, _, _, 0, IsSc | IsXc) => Sc(Sci::DrawLarge { x, y }),
            (0xD, _, _, _, _) => Self::Drw { x, y, n },
            (0xE, _, 0x9, 0xE, _) => Self::Skp { x },
            (0xE, _, 0xA, 0x1, _) => Self::Sknp { x },
            (0xF, 0x0, 0x0, 0x0, IsXc) => Xc(Xci::LdILong),
            (0xF, _, 0x0, 0x1, IsXc) => Xc(Xci::SelectPlanes { x }),
            (0xF, 0x0, 0x0, 0x2, IsXc) => Xc(Xci::WriteAudio),
            (0xF, _, 0x0, 0x7, _) => Self::LdFromDt { x },
            (0xF, _, 0x0, 0xA, _) => Self::LdFromKey { x },
            (0xF, _, 0x1, 0x5, _) => Self::LdToDt { x },
            (0xF, _, 0x1, 0x8, _) => Self::LdSt { x },
            (0xF, _, 0x1, 0xE, _) => Self::AddI { x },
            (0xF, _, 0x2, 0x9, _) => Self::LdF { x },
            (0xF, _, 0x3, 0x0, IsSc | IsXc) => Sc(Sci::LdHiResF { x }),
            (0xF, _, 0x3, 0x3, _) => Self::LdB { x },
            (0xF, _, 0x3, 0xA, IsXc) => Xc(Xci::SetPitch { x }),
            (0xF, _, 0x5, 0x5, _) => Self::LdToSlice { x },
            (0xF, _, 0x6, 0x5, _) => Self::LdFromSlice { x },
            (0xF, _, 0x7, 0x5, IsSc | IsXc) => Sc(Sci::StoreRegs { x }),
            (0xF, _, 0x8, 0x5, IsSc | IsXc) => Sc(Sci::GetRegs { x }),
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
