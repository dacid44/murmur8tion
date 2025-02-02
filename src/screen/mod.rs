mod cosmac_vip;
mod schip;
mod xochip;

use std::{
    cmp::Ordering,
    fmt::Binary,
    mem,
    ops::{BitAnd, BitXorAssign, Shl, ShlAssign, Shr},
};

use image::{Rgba, RgbaImage};
use num_traits::PrimInt;
use thiserror::Error;
use ux::u4;

pub use cosmac_vip::CosmacVipScreen;
pub use schip::{LegacySuperChipScreen, ModernSuperChipScreen};
pub use xochip::XoChipScreen;

// from https://github.com/gulrak/cadmium/blob/1e1f524c4d1c5ceff3b3da8818f0ed815e9160db/src/cadmium.cpp#L1893-L1898
const CADMIUM_PALETTE: [u32; 16] = [
    0x1a1c2cff, 0xf4f4f4ff, 0x94b0c2ff, 0x333c57ff, 0xb13e53ff, 0xa7f070ff, 0x3b5dc9ff, 0xffcd75ff,
    0x5d275dff, 0x38b764ff, 0x29366fff, 0x566c86ff, 0xef7d57ff, 0x73eff7ff, 0x41a6f6ff, 0x257179ff,
];

pub struct Palette {
    pub two_color: [Rgba<u8>; 2],
    pub sixteen_color: [Rgba<u8>; 16],
    pub use_custom_two_color: bool,
}

impl Default for Palette {
    fn default() -> Self {
        let sixteen_color = CADMIUM_PALETTE.map(|color| Rgba::from(color.to_be_bytes()));
        Self {
            two_color: [sixteen_color[0], sixteen_color[1]],
            sixteen_color,
            use_custom_two_color: true,
        }
    }
}

impl Palette {
    fn two_color_off(&self) -> Rgba<u8> {
        if self.use_custom_two_color {
            self.two_color[0]
        } else {
            self.sixteen_color[0]
        }
    }

    fn two_color_on(&self) -> Rgba<u8> {
        if self.use_custom_two_color {
            self.two_color[1]
        } else {
            self.sixteen_color[1]
        }
    }
}

#[derive(Error, Debug)]
pub enum UnsupportedScreenOperation {
    #[error("this screen type does not support hires mode")]
    HiresMode,
    #[error("this screen type does not support multiple display planes")]
    SetPlanes,
    #[error("large sprites are not supported with this screen type")]
    LargeSprite,
    #[error("large sprites are not supported in lores mode on with this screen type")]
    LargeSpriteInLores,
    #[error("scrolling (down) is not supported with this screen type")]
    ScrollDown,
    #[error("scrolling (up) is not supported with this screen type")]
    ScrollUp,
    #[error("scrolling (right) is not supported with this screen type")]
    ScrollRight,
    #[error("scrolling (left) is not supported with this screen type")]
    ScrollLeft,
}

type Result<T, E = UnsupportedScreenOperation> = std::result::Result<T, E>;

pub trait Screen {
    fn width(&self) -> u8;
    fn height(&self) -> u8;
    fn clear(&mut self);
    fn get_hires(&self) -> bool {
        false
    }
    fn set_hires(&mut self, _hires: bool) -> Result<()> {
        Err(UnsupportedScreenOperation::HiresMode)
    }
    fn set_planes(&mut self, _planes: u4) -> Result<()> {
        Err(UnsupportedScreenOperation::SetPlanes)
    }
    fn num_active_planes(&self) -> usize {
        1
    }
    fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool;
    fn draw_large_sprite(&mut self, _x: u8, _y: u8, _sprite: &[[u8; 32]]) -> Result<u8> {
        Err(UnsupportedScreenOperation::LargeSprite)
    }
    fn scroll_down(&mut self, _amount: u4) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollDown)
    }
    fn scroll_up(&mut self, _amount: u4) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollUp)
    }
    fn scroll_right(&mut self) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollRight)
    }
    fn scroll_left(&mut self) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollLeft)
    }
    fn to_image(&self, palette: &Palette) -> RgbaImage;
}

macro_rules! screen_method {
    ($name:ident(self: $($selfty:ty)?$(, $param:ident: $ptype:ty)*)$( -> $ret:ty)?) => {
        fn $name(self$(: $selfty)?$(, $param: $ptype)*)$( -> $ret)? {
            match self {
                Self::CosmacVip(screen) => Screen::$name(screen$(, $param)*),
                Self::LegacySuperChip(screen) => Screen::$name(screen$(, $param)*),
                Self::ModernSuperChip(screen) => Screen::$name(screen$(, $param)*),
                Self::XoChip(screen) => Screen::$name(screen$(, $param)*),
            }
        }
    }
}

pub enum DynamicScreen {
    CosmacVip(CosmacVipScreen),
    LegacySuperChip(LegacySuperChipScreen),
    ModernSuperChip(ModernSuperChipScreen),
    XoChip(XoChipScreen),
}

impl Screen for DynamicScreen {
    screen_method!(width(self: &Self) -> u8);
    screen_method!(height(self: &Self) -> u8);
    screen_method!(clear(self: &mut Self));
    screen_method!(get_hires(self: &Self) -> bool);
    screen_method!(set_hires(self: &mut Self, hires: bool) -> Result<()>);
    screen_method!(set_planes(self: &mut Self, planes: u4) -> Result<()>);
    screen_method!(num_active_planes(self: &Self) -> usize);
    screen_method!(draw_sprite(self: &mut Self, x: u8, y: u8, sprite: &[u8]) -> bool);
    screen_method!(draw_large_sprite(self: &mut Self, x: u8, y: u8, sprite: &[[u8; 32]]) -> Result<u8>);
    screen_method!(scroll_down(self: &mut Self, amount: u4) -> Result<()>);
    screen_method!(scroll_up(self: &mut Self, amount: u4) -> Result<()>);
    screen_method!(scroll_right(self: &mut Self) -> Result<()>);
    screen_method!(scroll_left(self: &mut Self) -> Result<()>);
    screen_method!(to_image(self: &Self, palette: &Palette) -> RgbaImage);
}

pub const FONT_ADDRESS: usize = 0;
pub const FONT: [[u8; 5]; 16] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0],
    [0x20, 0x60, 0x20, 0x20, 0x70],
    [0xF0, 0x10, 0xF0, 0x80, 0xF0],
    [0xF0, 0x10, 0xF0, 0x10, 0xF0],
    [0x90, 0x90, 0xF0, 0x10, 0x10],
    [0xF0, 0x80, 0xF0, 0x10, 0xF0],
    [0xF0, 0x80, 0xF0, 0x90, 0xF0],
    [0xF0, 0x10, 0x20, 0x40, 0x40],
    [0xF0, 0x90, 0xF0, 0x90, 0xF0],
    [0xF0, 0x90, 0xF0, 0x10, 0xF0],
    [0xF0, 0x90, 0xF0, 0x90, 0x90],
    [0xE0, 0x90, 0xE0, 0x90, 0xE0],
    [0xF0, 0x80, 0x80, 0x80, 0xF0],
    [0xE0, 0x90, 0x90, 0x90, 0xE0],
    [0xF0, 0x80, 0xF0, 0x80, 0xF0],
    [0xF0, 0x80, 0xF0, 0x80, 0x80],
];

pub const SCHIP_HIRES_FONT_ADDRESS: usize = FONT_ADDRESS + 80;
pub const SCHIP_HIRES_FONT: [[u8; 10]; 10] = [
    [0x3C, 0x7E, 0xE7, 0xC3, 0xC3, 0xC3, 0xC3, 0xE7, 0x7E, 0x3C],
    [0x18, 0x38, 0x58, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C],
    [0x3E, 0x7F, 0xC3, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xFF, 0xFF],
    [0x3C, 0x7E, 0xC3, 0x03, 0x0E, 0x0E, 0x03, 0xC3, 0x7E, 0x3C],
    [0x06, 0x0E, 0x1E, 0x36, 0x66, 0xC6, 0xFF, 0xFF, 0x06, 0x06],
    [0xFF, 0xFF, 0xC0, 0xC0, 0xFC, 0xFE, 0x03, 0xC3, 0x7E, 0x3C],
    [0x3E, 0x7C, 0xE0, 0xC0, 0xFC, 0xFE, 0xC3, 0xC3, 0x7E, 0x3C],
    [0xFF, 0xFF, 0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x60, 0x60],
    [0x3C, 0x7E, 0xC3, 0xC3, 0x7E, 0x7E, 0xC3, 0xC3, 0x7E, 0x3C],
    [0x3C, 0x7E, 0xC3, 0xC3, 0x7F, 0x3F, 0x03, 0x03, 0x3E, 0x7C],
];

pub const XOCHIP_HIRES_FONT_ADDRESS: usize = FONT_ADDRESS + 80;
pub const XOCHIP_HIRES_FONT: [[u8; 10]; 16] = [
    [0x3C, 0x7E, 0x66, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x7E, 0x3C],
    [0x0C, 0x1C, 0x3C, 0x6C, 0x0C, 0x0C, 0x0C, 0x0C, 0x7E, 0x7E],
    [0x3C, 0x7E, 0x66, 0x06, 0x0E, 0x1C, 0x38, 0x70, 0x7E, 0x7E],
    [0x3C, 0x7E, 0x66, 0x06, 0x1C, 0x1C, 0x06, 0x66, 0x7E, 0x3C],
    [0x6C, 0x6C, 0x6C, 0x6C, 0x7E, 0x7E, 0x0C, 0x0C, 0x0C, 0x0C],
    [0x7C, 0x7C, 0x60, 0x60, 0x7C, 0x3E, 0x06, 0x66, 0x7E, 0x3C],
    [0x3C, 0x7E, 0x66, 0x60, 0x7C, 0x7E, 0x66, 0x66, 0x7E, 0x3C],
    [0x7E, 0x7E, 0x06, 0x0E, 0x0C, 0x18, 0x18, 0x30, 0x30, 0x30],
    [0x3C, 0x7E, 0x66, 0x66, 0x3C, 0x3C, 0x66, 0x66, 0x7E, 0x3C],
    [0x3C, 0x7E, 0x66, 0x66, 0x7E, 0x3E, 0x06, 0x66, 0x7E, 0x3C],
    [0x3C, 0x7E, 0x66, 0x66, 0x7E, 0x7E, 0x66, 0x66, 0x66, 0x66],
    [0x7C, 0x7E, 0x66, 0x66, 0x7C, 0x7C, 0x66, 0x66, 0x7E, 0x7C],
    [0x3C, 0x7E, 0x66, 0x66, 0x60, 0x60, 0x66, 0x66, 0x7E, 0x3C],
    [0x7C, 0x7E, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7E, 0x7C],
    [0x7E, 0x7E, 0x60, 0x60, 0x78, 0x78, 0x60, 0x60, 0x7E, 0x7E],
    [0x7E, 0x7E, 0x60, 0x60, 0x78, 0x78, 0x60, 0x60, 0x60, 0x60],
];

fn draw_line_clipping<D, L>(dest: &mut D, x: u8, line: L) -> bool
where
    D: Copy
        + From<L>
        + From<u8>
        + Shl<u8, Output = D>
        + Shr<u8, Output = D>
        + BitXorAssign
        + BitAnd<Output = D>
        + PartialEq,
{
    draw_line(dest, x, line, Shl::shl, Shr::shr)
}

fn draw_line<D, L, LF, RF>(dest: &mut D, x: u8, line: L, lshift: LF, rshift: RF) -> bool
where
    D: Copy + From<L> + From<u8> + BitXorAssign + BitAnd<Output = D> + PartialEq,
    LF: Fn(D, u8) -> D,
    RF: Fn(D, u8) -> D,
{
    let width_difference = (mem::size_of::<D>() - mem::size_of::<L>()) as u8 * 8;
    let line = D::from(line);
    let mask = match x.cmp(&width_difference) {
        Ordering::Less => lshift(line, width_difference - x),
        Ordering::Equal => line,
        Ordering::Greater => rshift(line, x - width_difference),
    };

    let collided = *dest & mask != D::from(0);
    *dest ^= mask;
    collided
}

fn screen_to_image<N: PrimInt + ShlAssign<u32> + Binary>(
    data: &[N],
    palette: &Palette,
) -> RgbaImage {
    let width = mem::size_of::<N>() as u32 * 8;
    let mut image = RgbaImage::from_pixel(width, data.len() as u32, palette.two_color_off());
    let on_color = palette.two_color_on();
    for (i, line) in data.iter().enumerate() {
        // println!("\nline {i} {line:#066b}");
        let mut shift = 0;
        let mut line = *line;
        loop {
            let leading_zeros = line.leading_zeros();
            if leading_zeros >= width as u32 {
                break;
            }
            shift += leading_zeros + 1;
            // print!("; {leading_zeros} {shift}");
            image.put_pixel(shift - 1, i as u32, on_color);
            if leading_zeros + 1 >= width as u32 {
                break;
            }
            line <<= leading_zeros + 1;
        }
    }
    image
}

/// Double each bit in x.
/// Credit to https://stackoverflow.com/a/2929404
/// Based on https://graphics.stanford.edu/~seander/bithacks.html#Interleave64bitOps
fn double_bits_holger(x: u8) -> u16 {
    let m = ((x as u64).wrapping_mul(0x0101010101010101) & 0x8040201008040201)
        .wrapping_mul(0x0102040810204081);
    // (((m >> 49) & 0x5555) | ((m >> 48) & 0xAAAA)) as u16
    (((m >> 49) & 0x5555) * 3) as u16
}

#[test]
fn test_double_bits_holger() {
    assert_eq!(double_bits_holger(0b10101010), 0b1100110011001100);
    assert_eq!(double_bits_holger(0b01010101), 0b0011001100110011);
    assert_eq!(double_bits_holger(0b10010101), 0b1100001100110011);
    assert_eq!(double_bits_holger(0), 0);
    assert_eq!(double_bits_holger(0xFF), 0xFFFF);
}

/// Double each bit in x.
/// Credit to https://stackoverflow.com/a/2929404
/// Based on https://graphics.stanford.edu/~seander/bithacks.html#InterleaveBMN
fn double_bits_magic(x: u16) -> u32 {
    let mut x = x as u32;
    x = (x | x << 8) & 0x00FF00FF;
    x = (x | x << 4) & 0x0F0F0F0F;
    x = (x | x << 2) & 0x33333333;
    x = (x | x << 1) & 0x55555555;
    x | x << 1
}

#[test]
fn test_double_bits_magic() {
    assert_eq!(
        double_bits_magic(0b1010101010101010),
        0b11001100110011001100110011001100
    );
    assert_eq!(
        double_bits_magic(0b0101010101010101),
        0b00110011001100110011001100110011
    );
    assert_eq!(
        double_bits_magic(0b1001010100110110),
        0b11000011001100110000111100111100
    );
    assert_eq!(double_bits_magic(0), 0);
    assert_eq!(double_bits_magic(0xFFFF), 0xFFFFFFFF);
}

// 000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000abcdefghijklmnopqrstuvwxyz345678    x
// 000000000000000000000000000000000000000000000000abcdefghijklmnop000000000000000000000000000000000000000000000000qrstuvwxyz345678    x = (x | x << 48) & 0x000000000000FFFF000000000000FFFF
// 000000000000000000000000abcdefgh000000000000000000000000ijklmnop000000000000000000000000qrstuvwx000000000000000000000000yz345678    x = (x | x << 24) & 0x000000FF000000FF000000FF000000FF
// 000000000000abcd000000000000efgh000000000000ijkl000000000000mnop000000000000qrst000000000000uvwx000000000000yz340000000000005678    x = (x | x << 12) & 0x000F000F000F000F000F000F000F000F
// 000000ab000000cd000000ef000000gh000000ij000000kl000000mn000000op000000qr000000st000000uv000000wx000000yz000000340000005600000078    x = (x | x << 6)  & 0x03030303030303030303030303030303
// 000a000b000c000d000e000f000g000h000i000j000k000l000m000n000o000p000q000r000s000t000u000v000w000x000y000z000300040005000600070008    x = (x | x << 3)  & 0x11111111111111111111111111111111
fn expand_32bit_4x(x: u32) -> u128 {
    let mut x = x as u128;
    x = (x | x << 48) & 0x000000000000FFFF000000000000FFFF;
    x = (x | x << 24) & 0x000000FF000000FF000000FF000000FF;
    x = (x | x << 12) & 0x000F000F000F000F000F000F000F000F;
    x = (x | x << 6) & 0x03030303030303030303030303030303;
    x = (x | x << 3) & 0x11111111111111111111111111111111;
    x
}

fn expand_u32(x: u32) -> u64 {
    let mut x = x as u64;
    x = (x | x << 16) & 0x0000FFFF0000FFFF;
    x = (x | x << 8) & 0x00FF00FF00FF00FF;
    x = (x | x << 4) & 0x0F0F0F0F0F0F0F0F;
    x = (x | x << 2) & 0x3333333333333333;
    x = (x | x << 1) & 0x5555555555555555;
    x
}

fn expand_u64(x: u64) -> u128 {
    let mut x = x as u128;
    x = (x | x << 32) & 0x00000000FFFFFFFF00000000FFFFFFFF;
    x = (x | x << 16) & 0x0000FFFF0000FFFF0000FFFF0000FFFF;
    x = (x | x << 8) & 0x00FF00FF00FF00FF00FF00FF00FF00FF;
    x = (x | x << 4) & 0x0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F;
    x = (x | x << 2) & 0x33333333333333333333333333333333;
    x = (x | x << 1) & 0x55555555555555555555555555555555;
    x
}

fn u64_nibbles(x: u64) -> [u8; 16] {
    let mut x = x as u128;
    x = (x | x << 32) & 0x00000000FFFFFFFF00000000FFFFFFFF;
    x = (x | x << 16) & 0x0000FFFF0000FFFF0000FFFF0000FFFF;
    x = (x | x << 8) & 0x00FF00FF00FF00FF00FF00FF00FF00FF;
    x = (x | x << 4) & 0x0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F;
    x.to_be_bytes()
}

fn combine_plane_segments(a: u32, b: u32, c: u32, d: u32) -> [u8; 32] {
    let ac = expand_u32(a) << 1 | expand_u32(c);
    let bd = expand_u32(b) << 1 | expand_u32(d);
    let abcd = expand_u64(ac) << 1 | expand_u64(bd);
    bytemuck::must_cast([
        u64_nibbles((abcd >> 64) as u64),
        u64_nibbles((abcd & 0xFFFFFFFFFFFFFFFF) as u64),
    ])
}

#[cfg(target_endian = "big")]
fn split_u128_4x(x: u128) -> [u32; 4] {
    bytemuck::must_cast(x)
}

#[cfg(target_endian = "little")]
fn split_u128_4x(x: u128) -> [u32; 4] {
    let [a, b, c, d] = bytemuck::must_cast(x);
    [d, c, b, a]
}

fn combine_planes(a: u128, b: u128, c: u128, d: u128) -> [u8; 128] {
    let [aa, ab, ac, ad] = split_u128_4x(a);
    let [ba, bb, bc, bd] = split_u128_4x(b);
    let [ca, cb, cc, cd] = split_u128_4x(c);
    let [da, db, dc, dd] = split_u128_4x(d);
    bytemuck::must_cast([
        combine_plane_segments(aa, ba, ca, da),
        combine_plane_segments(ab, bb, cb, db),
        combine_plane_segments(ac, bc, cc, dc),
        combine_plane_segments(ad, bd, cd, dd),
    ])
}
