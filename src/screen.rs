use std::{
    cmp::Ordering,
    mem,
    ops::{BitAnd, BitOr, BitXorAssign, Shl, ShlAssign, Shr},
};

use image::{Rgba, RgbaImage};
use num_traits::{PrimInt, Zero};
use thiserror::Error;
use ux::u4;

const ON_COLOR: Rgba<u8> = Rgba([0, 100, 0, 255]);
const OFF_COLOR: Rgba<u8> = Rgba([0, 0, 0, 255]);

#[derive(Error, Debug)]
pub enum UnsupportedScreenOperation {
    #[error("this screen type does not support hires mode")]
    HiresMode,
    #[error("large sprites are not supported with this screen type")]
    LargeSprite,
    #[error("scrolling (down) is not supported with this screen type")]
    ScrollDown,
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
    fn set_hires(&mut self, hires: bool) -> Result<()>;
    fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool;
    fn draw_large_sprite(&mut self, x: u8, y: u8, sprite: &[u8; 32]) -> Result<u8>;
    fn scroll_down(&mut self, amount: u4) -> Result<()>;
    fn scroll_right(&mut self) -> Result<()>;
    fn scroll_left(&mut self) -> Result<()>;
    fn to_image(&self) -> RgbaImage;
}

#[derive(Default)]
pub struct CosmacVipScreen(Box<[u64; 32]>);

// impl Default for CosmacVipScreen {
//     fn default() -> Self {
//         Self(Box::new([0xFF00FF00FF00FF00; 32]))
//     }
// }

impl CosmacVipScreen {
    pub const WIDTH: u8 = 64;
    pub const HEIGHT: u8 = 32;
}

impl Screen for CosmacVipScreen {
    fn width(&self) -> u8 {
        Self::WIDTH
    }

    fn height(&self) -> u8 {
        Self::HEIGHT
    }

    fn clear(&mut self) {
        self.0 = Default::default();
    }

    fn set_hires(&mut self, _hires: bool) -> Result<()> {
        Err(UnsupportedScreenOperation::HiresMode)
    }

    fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        sprite
            .iter()
            .zip(self.0[(y % Self::HEIGHT) as usize..].iter_mut())
            .map(|(line, dest)| draw_line(dest, x % Self::WIDTH, *line))
            .fold(false, BitOr::bitor)
    }

    fn draw_large_sprite(&mut self, _x: u8, _y: u8, _sprite: &[u8; 32]) -> Result<u8> {
        Err(UnsupportedScreenOperation::LargeSprite)
    }

    fn scroll_down(&mut self, _amount: u4) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollDown)
    }

    fn scroll_right(&mut self) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollRight)
    }

    fn scroll_left(&mut self) -> Result<()> {
        Err(UnsupportedScreenOperation::ScrollLeft)
    }

    fn to_image(&self) -> RgbaImage {
        screen_to_image(self.0.as_slice())
    }
}

pub struct SuperChipScreen {
    data: Box<[u128; 64]>,
    hires: bool,
}

impl Default for SuperChipScreen {
    fn default() -> Self {
        Self {
            data: Box::new([0; 64]),
            hires: false,
        }
    }
}

impl SuperChipScreen {
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;
}

impl Screen for SuperChipScreen {
    fn width(&self) -> u8 {
        Self::WIDTH
    }

    fn height(&self) -> u8 {
        Self::HEIGHT
    }

    fn clear(&mut self) {
        self.data = Box::new([0; 64]);
    }

    fn set_hires(&mut self, hires: bool) -> Result<()> {
        self.hires = hires;
        Ok(())
    }

    fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        if self.hires {
            sprite
                .iter()
                .zip(self.data[(y % Self::HEIGHT) as usize..].iter_mut())
                .map(|(line, dest)| draw_line(dest, x % Self::WIDTH, *line))
                .fold(false, BitOr::bitor)
        } else {
            let x = x * 2 % Self::WIDTH;
            sprite
                .iter()
                .copied()
                .map(double_bits_holger)
                .zip(self.data[(y * 2 % Self::HEIGHT) as usize..].chunks_exact_mut(2))
                .map(|(line, dest)| {
                    draw_line(&mut dest[0], x, line) | draw_line(&mut dest[1], x, line)
                })
                .fold(false, BitOr::bitor)
        }
    }

    fn draw_large_sprite(&mut self, x: u8, y: u8, sprite: &[u8; 32]) -> Result<u8> {
        let collided = if self.hires {
            sprite
                .chunks_exact(2)
                .map(|line| u16::from_be_bytes([line[0], line[1]]))
                .zip(self.data[(y % Self::HEIGHT) as usize..].iter_mut())
                .map(|(line, dest)| draw_line(dest, x, line) as u8)
                .sum()
        } else {
            let x = x * 2 % Self::WIDTH;
            sprite
                .chunks_exact(2)
                .map(|line| u16::from_be_bytes([line[0], line[1]]))
                .map(double_bits_magic)
                .zip(self.data[(y * 2 % Self::HEIGHT) as usize..].chunks_exact_mut(2))
                .map(|(line, dest)| {
                    draw_line(&mut dest[0], x, line) as u8 + draw_line(&mut dest[1], x, line) as u8
                })
                .sum()
        };
        Ok(collided)
    }

    fn scroll_down(&mut self, amount: u4) -> Result<()> {
        let amount = u8::from(if self.hires { amount >> 1 } else { amount }) as usize;
        if amount > 0 {
            self.data
                .copy_within(..Self::HEIGHT as usize - amount, amount);
            for line in self.data[..amount].iter_mut() {
                *line = 0;
            }
        }
        Ok(())
    }

    fn scroll_right(&mut self) -> Result<()> {
        for line in self.data.iter_mut() {
            *line >>= 4;
        }
        Ok(())
    }

    fn scroll_left(&mut self) -> Result<()> {
        for line in self.data.iter_mut() {
            *line <<= 4;
        }
        Ok(())
    }

    fn to_image(&self) -> RgbaImage {
        screen_to_image(self.data.as_slice())
    }
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

pub const SCHIP_HIRES_FONT_ADDRESS: usize = 0x100;
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

fn draw_line<D, L>(dest: &mut D, x: u8, line: L) -> bool
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
    let width_difference = (mem::size_of::<D>() - mem::size_of::<L>()) as u8 * 8;
    let line = D::from(line);
    let mask = match x.cmp(&width_difference) {
        Ordering::Less => line << (width_difference - x),
        Ordering::Equal => line,
        Ordering::Greater => line >> (x - width_difference),
    };

    let collided = *dest & mask != D::from(0);
    *dest ^= mask;
    collided
}

fn screen_to_image<N: PrimInt + ShlAssign<u32>>(data: &[N]) -> RgbaImage {
    let width = mem::size_of::<N>() as u32 * 8;
    let mut image = RgbaImage::from_pixel(width, data.len() as u32, OFF_COLOR);
    for (i, line) in data.iter().enumerate() {
        let mut shift = 0;
        let mut line = *line;
        loop {
            let leading_zeros = line.leading_zeros();
            shift += leading_zeros + 1;
            line <<= leading_zeros + 1;
            if shift >= width as u32 {
                break;
            }
            image.put_pixel(shift - 1, i as u32, ON_COLOR);
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
