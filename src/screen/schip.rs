use std::ops::BitOr;

use arbitrary_int::u4;
use bytemuck::Zeroable;
use image::RgbaImage;

use super::{
    double_bits_holger, double_bits_magic, draw_line_clipping, screen_to_image, Palette, Result,
    Screen,
};

#[derive(Clone, Zeroable)]
pub struct LegacySuperChipScreen {
    data: [u128; 64],
    hires: bool,
}

impl LegacySuperChipScreen {
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;
}

impl Default for Box<LegacySuperChipScreen> {
    fn default() -> Self {
        bytemuck::zeroed_box()
    }
}

impl Screen for LegacySuperChipScreen {
    fn width(&self) -> u8 {
        Self::WIDTH
    }

    fn height(&self) -> u8 {
        Self::HEIGHT
    }

    fn clear(&mut self) {
        bytemuck::fill_zeroes(&mut self.data);
    }

    fn get_hires(&self) -> bool {
        self.hires
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
                .map(|(line, dest)| draw_line_clipping(dest, x % Self::WIDTH, *line))
                .fold(false, BitOr::bitor)
        } else {
            let x = (x << 1) % Self::WIDTH;
            let zone_offset = x & 0xF0;
            let mask: u128 = 0xFFFFFFFF_00000000_00000000_00000000 >> zone_offset;
            sprite
                .iter()
                .copied()
                .map(double_bits_holger)
                .zip(self.data[((y << 1) % Self::HEIGHT) as usize..].chunks_exact_mut(2))
                .map(|(line, dest)| {
                    let collided = draw_line_clipping(&mut dest[0], x, line);
                    dest[1] = (dest[1] & !mask) | (dest[0] & mask);
                    collided
                })
                .fold(false, BitOr::bitor)
        }
    }

    fn draw_large_sprite(&mut self, x: u8, y: u8, sprite: &[[u8; 32]]) -> Result<u8> {
        let collided = if self.hires {
            sprite[0]
                .chunks_exact(2)
                .map(|line| u16::from_be_bytes([line[0], line[1]]))
                .zip(self.data[(y % Self::HEIGHT) as usize..].iter_mut())
                .map(|(line, dest)| draw_line_clipping(dest, x % Self::WIDTH, line) as u8)
                .sum()
        } else {
            return Err(super::UnsupportedScreenOperation::LargeSpriteInLores);
        };
        Ok(collided)
    }

    fn scroll_down(&mut self, amount: u4) -> Result<()> {
        let amount = u8::from(amount) as usize;
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

    fn to_image(&self, palette: &Palette) -> RgbaImage {
        screen_to_image(self.data.as_slice(), palette)
    }
}

#[derive(Clone, Zeroable)]
pub struct ModernSuperChipScreen {
    data: [u128; 64],
    hires: bool,
}

impl ModernSuperChipScreen {
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;
}

impl Default for Box<ModernSuperChipScreen> {
    fn default() -> Self {
        bytemuck::zeroed_box()
    }
}

impl Screen for ModernSuperChipScreen {
    fn width(&self) -> u8 {
        Self::WIDTH
    }

    fn height(&self) -> u8 {
        Self::HEIGHT
    }

    fn clear(&mut self) {
        bytemuck::fill_zeroes(&mut self.data);
    }

    fn get_hires(&self) -> bool {
        self.hires
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
                .map(|(line, dest)| draw_line_clipping(dest, x % Self::WIDTH, *line))
                .fold(false, BitOr::bitor)
        } else {
            let x = (x << 1) % Self::WIDTH;
            sprite
                .iter()
                .copied()
                .map(double_bits_holger)
                .zip(self.data[((y << 1) % Self::HEIGHT) as usize..].chunks_exact_mut(2))
                .map(|(line, dest)| {
                    draw_line_clipping(&mut dest[0], x, line)
                        | draw_line_clipping(&mut dest[1], x, line)
                })
                .fold(false, BitOr::bitor)
        }
    }

    fn draw_large_sprite(&mut self, x: u8, y: u8, sprite: &[[u8; 32]]) -> Result<u8> {
        let collided = if self.hires {
            sprite[0]
                .chunks_exact(2)
                .map(|line| u16::from_be_bytes([line[0], line[1]]))
                .zip(self.data[(y % Self::HEIGHT) as usize..].iter_mut())
                .map(|(line, dest)| draw_line_clipping(dest, x % Self::WIDTH, line))
                .fold(false, BitOr::bitor)
        } else {
            let x = (x << 1) % Self::WIDTH;
            sprite[0]
                .chunks_exact(2)
                .map(|line| u16::from_be_bytes([line[0], line[1]]))
                .map(double_bits_magic)
                .zip(self.data[((y << 1) % Self::HEIGHT) as usize..].chunks_exact_mut(2))
                .map(|(line, dest)| {
                    draw_line_clipping(&mut dest[0], x, line)
                        | draw_line_clipping(&mut dest[1], x, line)
                })
                .fold(false, BitOr::bitor)
        };
        Ok(collided as u8)
    }

    fn scroll_down(&mut self, amount: u4) -> Result<()> {
        let mut amount = u8::from(amount) as usize;
        if !self.hires {
            amount *= 2;
        }
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
        let amount = if self.hires { 4 } else { 8 };
        for line in self.data.iter_mut() {
            *line >>= amount;
        }
        Ok(())
    }

    fn scroll_left(&mut self) -> Result<()> {
        let amount = if self.hires { 4 } else { 8 };
        for line in self.data.iter_mut() {
            *line <<= amount;
        }
        Ok(())
    }

    fn to_image(&self, palette: &Palette) -> RgbaImage {
        screen_to_image(self.data.as_slice(), palette)
    }
}
