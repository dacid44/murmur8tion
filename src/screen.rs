use std::cmp::Ordering;

use egui_macroquad::macroquad::texture::Image;

pub trait Screen {
    fn width(&self) -> u8;
    fn height(&self) -> u8;
    fn clear(&mut self);
    fn draw_byte(&mut self, x: u8, y: u8, byte: u8) -> bool;
    fn to_image(&self) -> Image;
}

pub struct CosmacVipScreen(Box<[u64; 32]>);

impl Default for CosmacVipScreen {
    fn default() -> Self {
        Self(Box::new([0xFF00FF00FF00FF00; 32]))
    }
}

impl CosmacVipScreen {
    const WIDTH: u8 = 64;
    const HEIGHT: u8 = 32;
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

    fn draw_byte(&mut self, x: u8, y: u8, byte: u8) -> bool {
        let x = x % Self::WIDTH;
        let y = y % Self::HEIGHT;

        let mask = match x.cmp(&(Self::WIDTH - 8)) {
            Ordering::Less => (byte as u64) << (Self::WIDTH - 8 - x),
            Ordering::Equal => byte as u64,
            Ordering::Greater => (byte as u64) >> (x - (Self::WIDTH - 8)),
        };

        let erased = self.0[y as usize] & mask != 0;
        self.0[y as usize] ^= mask;
        erased
    }

    fn to_image(&self) -> Image {
        eprintln!("{:?}", self.0);
        let mut bytes = vec![255; Self::WIDTH as usize * Self::HEIGHT as usize * 4];
        for (i, line) in self.0.iter().enumerate() {
            let mut shift = 0;
            let mut line = *line;
            while shift < Self::WIDTH as u32 {
                let leading_ones = line.leading_ones();
                shift += leading_ones + 1;
                line <<= leading_ones + 1;
                let pixel_start = (i * Self::WIDTH as usize + shift as usize - 1) * 4;
                bytes[pixel_start..pixel_start + 3].copy_from_slice(&[0, 0, 0]);
            }
        }
        Image {
            bytes,
            width: Self::WIDTH as u16,
            height: Self::HEIGHT as u16,
        }
    }
}

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
