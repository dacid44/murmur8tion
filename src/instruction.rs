use arbitrary_int::{u12, u4};
use bitbybit::bitfield;

use crate::model::Quirks;

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

#[macro_export]
macro_rules! match_execute {
    ($type:ty, $self:ident, $x:ident, $y:ident, $n:ident, $x_u8:ident, $y_u8:ident, $n_u8:ident, $nn:ident, $nnn:ident; $return:expr; $($opcode:ident => $impl:expr)*) => {
        $(
            ::paste::paste! {
                #[inline(always)]
                #[allow(unreachable_code)]
                #[allow(unused_variables)]
                fn [<execute $opcode>](
                    $self: &mut Self,
                    $x: ::arbitrary_int::u4,
                    $y: ::arbitrary_int::u4,
                    $n: ::arbitrary_int::u4,
                    $x_u8: u8,
                    $y_u8: u8,
                    $n_u8: u8,
                    $nn: u8,
                    $nnn: u16,
                ) -> $type {
                    $impl;
                    $return
                }
            }
        )*
    };
    ($type:ty, $self:ident, $x:ident, $y:ident, $n:ident, $x_u8:ident, $y_u8:ident, $n_u8:ident, $nn:ident, $nnn:ident; $($opcode:ident => $impl:expr)*) => {
        $(
            ::paste::paste! {
                #[inline(always)]
                #[allow(unreachable_code)]
                #[allow(unused_variables)]
                fn [<execute $opcode>](
                    $self: &mut Self,
                    $x: ::arbitrary_int::u4,
                    $y: ::arbitrary_int::u4,
                    $n: ::arbitrary_int::u4,
                    $x_u8: u8,
                    $y_u8: u8,
                    $n_u8: u8,
                    $nn: u8,
                    $nnn: u16,
                ) -> $type {
                    $impl.into()
                }
            }
        )*
    };
}

#[allow(non_snake_case)]
#[allow(clippy::too_many_arguments)]
#[rustfmt::skip]
pub trait ExecuteInstruction<T> {
    #[inline(always)]
    fn execute(&mut self, opcode: u16, instruction_set: InstructionSet) -> T {
        use InstructionSet::SuperChip as IsSc;
        use InstructionSet::XoChip as IsXc;

        let [disc1, y_u8] = ((opcode & 0xF0F0) >> 4).to_be_bytes();
        let [x_u8, n_u8] = (opcode & 0x0F0F).to_be_bytes();
        let [x, y, n] = [x_u8, y_u8, n_u8].map(|nibble| unsafe { u4::new_unchecked(nibble) });
        let nn = (opcode & 0xFF) as u8;
        let nnn = opcode & 0xFFF;

        match (disc1, x_u8, y_u8, n_u8, instruction_set) {
            (0x10.., _, _, _, _)
            | (_, 0x10.., _, _, _)
            | (_, _, 0x10.., _, _)
            | (_, _, _, 0x10.., _) => unsafe { std::hint::unreachable_unchecked() },
            (0x0, 0x0, 0x0, 0x0, _) => self.execute_0000(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xC, _, IsSc | IsXc) => self.execute_00Cn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xD, _, IsXc) => self.execute_00Dn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xE, 0x0, _) => self.execute_00E0(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xE, 0xE, _) => self.execute_00EE(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xF, 0xB, IsSc | IsXc) => self.execute_00FB(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xF, 0xC, IsSc | IsXc) => self.execute_00FC(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xF, 0xD, IsSc | IsXc) => self.execute_00FD(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xF, 0xE, IsSc | IsXc) => self.execute_00FE(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x0, 0x0, 0xF, 0xF, IsSc | IsXc) => self.execute_00FF(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x1, _, _, _, _) => self.execute_1nnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x2, _, _, _, _) => self.execute_2nnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x3, _, _, _, _) => self.execute_3xnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x4, _, _, _, _) => self.execute_4xnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x5, _, _, 0x0, _) => self.execute_5xy0(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x5, _, _, 0x2, IsXc) => self.execute_5xy2(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x5, _, _, 0x3, IsXc) => self.execute_5xy3(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x6, _, _, _, _) => self.execute_6xnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x7, _, _, _, _) => self.execute_7xnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x0, _) => self.execute_8xy0(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x1, _) => self.execute_8xy1(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x2, _) => self.execute_8xy2(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x3, _) => self.execute_8xy3(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x4, _) => self.execute_8xy4(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x5, _) => self.execute_8xy5(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x6, _) => self.execute_8xy6(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0x7, _) => self.execute_8xy7(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x8, _, _, 0xE, _) => self.execute_8xyE(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0x9, _, _, 0x0, _) => self.execute_9xy0(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xA, _, _, _, _) => self.execute_Annn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xB, _, _, _, _) => self.execute_Bnnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xC, _, _, _, _) => self.execute_Cxnn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xD, _, _, 0, IsSc | IsXc) => self.execute_Dxy0(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xD, _, _, _, _) => self.execute_Dxyn(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xE, _, 0x9, 0xE, _) => self.execute_Ex9E(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xE, _, 0xA, 0x1, _) => self.execute_ExA1(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, 0x0, 0x0, 0x0, IsXc) => self.execute_F000(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x0, 0x1, IsXc) => self.execute_Fx01(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, 0x0, 0x0, 0x2, IsXc) => self.execute_F002(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x0, 0x7, _) => self.execute_Fx07(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x0, 0xA, _) => self.execute_Fx0A(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x1, 0x5, _) => self.execute_Fx15(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x1, 0x8, _) => self.execute_Fx18(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x1, 0xE, _) => self.execute_Fx1E(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x2, 0x9, _) => self.execute_Fx29(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x3, 0x0, IsSc | IsXc) => self.execute_Fx30(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x3, 0x3, _) => self.execute_Fx33(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x3, 0xA, IsXc) => self.execute_Fx3A(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x5, 0x5, _) => self.execute_Fx55(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x6, 0x5, _) => self.execute_Fx65(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x7, 0x5, IsSc | IsXc) => self.execute_Fx75(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            (0xF, _, 0x8, 0x5, IsSc | IsXc) => self.execute_Fx85(x, y, n, x_u8, y_u8, n_u8, nn, nnn),
            _ => self.no_match(opcode, x, y, n, x_u8, y_u8, n_u8, nn, nnn),
        }
    }

    fn execute_0000(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00Cn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00Dn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00E0(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00EE(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00FB(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00FC(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00FD(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00FE(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_00FF(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_1nnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_2nnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_3xnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_4xnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_5xy0(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_5xy2(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_5xy3(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_6xnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_7xnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy0(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy1(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy2(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy3(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy4(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy5(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy6(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xy7(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_8xyE(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_9xy0(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Annn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Bnnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Cxnn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Dxy0(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Dxyn(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Ex9E(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_ExA1(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_F000(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx01(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_F002(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx07(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx0A(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx15(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx18(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx1E(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx29(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx30(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx33(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx3A(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx55(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx65(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx75(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn execute_Fx85(&mut self, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
    fn no_match(&mut self, instruction: u16, x: u4, y: u4, n: u4, x_u8: u8, y_u8: u8, n_u8: u8, nn: u8, nnn: u16) -> T;
}

pub struct OctoSyntax<'a>(pub &'a Quirks, pub Option<u16>);

impl ExecuteInstruction<Option<String>> for OctoSyntax<'_> {
    match_execute! {Option<String>, self, x, y, n, x_u8, y_u8, n_u8, nn, nnn;
        _0000 => if self.0.graceful_exit_on_0000 {
            Some("exit-0000".to_owned())
        } else {
            None
        }
        _00Cn => format!("scroll-down {n:#X}")
        _00Dn => format!("scroll-up {n:#X}")
        _00E0 => "clear".to_owned()
        _00EE => "return".to_owned()
        _00FB => "scroll-right".to_owned()
        _00FC => "scroll-left".to_owned()
        _00FD => "exit".to_owned()
        _00FE => "lores".to_owned()
        _00FF => "hires".to_owned()
        _1nnn => format!("jump {nnn:#05X}")
        _2nnn => format!(":call {nnn:#05X}")
        _3xnn => format!("if v{x:X} != {nn:#04X} then")
        _4xnn => format!("if v{x:X} == {nn:#04X} then")
        _5xy0 => format!("if v{x:X} != v{y:X} then")
        _5xy2 => format!("save v{x:X} - v{y:X}")
        _5xy3 => format!("load v{x:X} - v{y:X}")
        _6xnn => format!("v{x:X} := {nn:#04X}")
        _7xnn => format!("v{x:X} += {nn:#04X}")
        _8xy0 => format!("v{x:X} := v{y:X}")
        _8xy1 => format!("v{x:X} |= v{y:X}")
        _8xy2 => format!("v{x:X} &= v{y:X}")
        _8xy3 => format!("v{x:X} ^= v{y:X}")
        _8xy4 => format!("v{x:X} += v{y:X}")
        _8xy5 => format!("v{x:X} -= v{y:X}")
        _8xy6 => format!("v{x:X} >>= {:X}", if self.0.bitshift_use_y { y } else { x })
        _8xy7 => format!("v{x:X} =- v{y:X}")
        _8xyE => format!("v{x:X} <<= {:X}", if self.0.bitshift_use_y { y } else { x })
        _9xy0 => format!("if v{x:X} == v{y:X} then")
        _Annn => format!("i := {nnn:#05X}")
        _Bnnn => if self.0.jump_v0_use_vx { format!("jump0 {nnn:#05X}") } else { format!("jump0 {nnn:#05X} + v{x:X}") }
        _Cxnn => format!("vX := random {nn:#04X}")
        _Dxy0 => format!("sprite v{x:X} v{y:X} 0")
        _Dxyn => format!("sprite v{x:X} v{y:X} {n:#X}")
        _Ex9E => format!("if v{x:X} -key then")
        _ExA1 => format!("if v{x:X} key then")
        _F000 => format!("i := long {}", match self.1.take() {
            Some(nnnn) => format!("{nnnn:#06X}"),
            None => "0x????".to_owned()
        })
        _Fx01 => format!("plane {x:#X}")
        _F002 => "audio".to_owned()
        _Fx07 => format!("v{x:X} := delay")
        _Fx0A => format!("v{x:X} := key")
        _Fx15 => format!("delay := v{x:X}")
        _Fx18 => format!("buzzer := v{x:X}")
        _Fx1E => format!("i += v{x:X}")
        _Fx29 => format!("i := hex v{x:X}")
        _Fx30 => format!("i := bighex v{x:X}")
        _Fx33 => format!("bcd v{x:X}")
        _Fx3A => format!("pitch := v{x:X}")
        _Fx55 => format!("save v{x:X}")
        _Fx65 => format!("load v{x:X}")
        _Fx75 => format!("saveflags v{x:X}")
        _Fx85 => format!("loadflags v{x:X}")
    }

    fn no_match(
        &mut self,
        _instruction: u16,
        _x: u4,
        _y: u4,
        _n: u4,
        _x_u8: u8,
        _y_u8: u8,
        _n_u8: u8,
        _nn: u8,
        _nnn: u16,
    ) -> Option<String> {
        None
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
