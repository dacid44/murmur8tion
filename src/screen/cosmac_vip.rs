use std::ops::BitOr;

use image::RgbaImage;

use super::{draw_line_clipping, screen_to_image, Palette, Screen};

#[derive(Default)]
pub struct CosmacVipScreen(Box<[u64; 32]>);

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

    fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        sprite
            .iter()
            .zip(self.0[(y % Self::HEIGHT) as usize..].iter_mut())
            .map(|(line, dest)| draw_line_clipping(dest, x % Self::WIDTH, *line))
            .fold(false, BitOr::bitor)
    }

    fn to_image(&self, palette: &Palette) -> RgbaImage {
        // println!("{:?}", self.0);
        screen_to_image(self.0.as_slice(), palette)
    }
}
