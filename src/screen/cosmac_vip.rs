use std::ops::BitOr;

use image::RgbaImage;
use ux::u4;

use super::{draw_line, screen_to_image, Palette, Result, Screen, UnsupportedScreenOperation};

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

    fn get_hires(&self) -> bool {
        false
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

    fn to_image(&self, palette: &Palette) -> RgbaImage {
        // println!("{:?}", self.0);
        screen_to_image(self.0.as_slice(), palette)
    }
}
