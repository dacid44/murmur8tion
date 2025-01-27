use std::cmp::Ordering;

pub trait Screen {
    fn width(&self) -> u8;
    fn height(&self) -> u8;
    fn clear(&mut self);
    fn draw_byte(&mut self, x: u8, y: u8, byte: u8) -> bool;
}

#[derive(Default)]
pub struct CosmacVipScreen(Box<[u64; 32]>);

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
