use std::ops::BitOr;

use image::RgbaImage;
use ux::u4;

use super::{
    combine_planes, double_bits_holger, double_bits_magic, draw_line, Palette, Result, Screen,
};

pub struct XoChipScreen {
    data: Box<[[u128; 64]; 4]>,
    enabled_planes: [bool; 4],
    hires: bool,
}

impl XoChipScreen {
    const WIDTH: u8 = 128;
    const HEIGHT: u8 = 64;

    fn iter_enabled_planes(&mut self) -> impl Iterator<Item = &mut [u128; 64]> {
        self.data
            .iter_mut()
            .zip(self.enabled_planes)
            .rev()
            .filter_map(|(plane, enabled)| enabled.then_some(plane))
    }
}

impl Default for XoChipScreen {
    fn default() -> Self {
        Self {
            data: bytemuck::zeroed_box(),
            enabled_planes: [false, false, false, true],
            hires: false,
        }
    }
}

impl Screen for XoChipScreen {
    fn width(&self) -> u8 {
        Self::WIDTH
    }

    fn height(&self) -> u8 {
        Self::HEIGHT
    }

    fn clear(&mut self) {
        for plane in self.iter_enabled_planes() {
            *plane = [0; 64];
        }
    }

    fn get_hires(&self) -> bool {
        self.hires
    }

    fn set_hires(&mut self, hires: bool) -> Result<()> {
        self.hires = hires;
        Ok(())
    }

    fn set_planes(&mut self, planes: u4) -> Result<()> {
        let planes = u8::from(planes);
        self.enabled_planes = [
            planes & 0b1000 != 0,
            planes & 0b0100 != 0,
            planes & 0b0010 != 0,
            planes & 0b0001 != 0,
        ];
        Ok(())
    }

    fn num_active_planes(&self) -> usize {
        self.enabled_planes
            .iter()
            .map(|plane| *plane as usize)
            .sum()
    }

    fn draw_sprite(&mut self, x: u8, y: u8, sprite: &[u8]) -> bool {
        // println!(
        //     "draw small sprite {x} {y} {}",
        //     sprite
        //         .iter()
        //         .map(|line| format!("{line:08b}"))
        //         .collect::<Vec<_>>()
        //         .join(" ")
        // );
        let hires = self.hires;
        let sprite_size = sprite.len() / self.num_active_planes();
        if hires {
            self.iter_enabled_planes()
                .zip(sprite.chunks(sprite_size))
                .flat_map(|(plane, sprite)| {
                    sprite
                        .iter()
                        .zip(iter_plane_wrapping(plane, y % Self::HEIGHT))
                        .map(|(line, dest)| draw_line_wrapping(dest, x % Self::WIDTH, *line))
                })
                .fold(false, BitOr::bitor)
        } else {
            let x = (x << 1) % Self::WIDTH;
            self.iter_enabled_planes()
                .zip(sprite.chunks(sprite_size))
                .flat_map(|(plane, sprite)| {
                    sprite
                        .iter()
                        .copied()
                        .map(double_bits_holger)
                        .zip(iter_plane_wrapping_pairs(plane, (y << 1) % Self::HEIGHT))
                        .map(|(line, dest)| {
                            draw_line_wrapping(&mut dest[0], x, line)
                                | draw_line_wrapping(&mut dest[1], x, line)
                        })
                })
                .fold(false, BitOr::bitor)
        }
    }

    fn draw_large_sprite(&mut self, x: u8, y: u8, sprite: &[[u8; 32]]) -> Result<u8> {
        let collided = if self.hires {
            self.iter_enabled_planes()
                .zip(sprite.iter())
                .flat_map(|(plane, sprite)| {
                    sprite
                        .chunks_exact(2)
                        .map(|line| u16::from_be_bytes([line[0], line[1]]))
                        .zip(iter_plane_wrapping(plane, y % Self::HEIGHT))
                        .map(|(line, dest)| draw_line_wrapping(dest, x % Self::WIDTH, line))
                })
                .fold(false, BitOr::bitor)
        } else {
            let x = (x << 1) % Self::WIDTH;
            self.iter_enabled_planes()
                .zip(sprite.iter())
                .flat_map(|(plane, sprite)| {
                    sprite
                        .chunks_exact(2)
                        .map(|line| u16::from_be_bytes([line[0], line[1]]))
                        .map(double_bits_magic)
                        .zip(iter_plane_wrapping_pairs(plane, (y << 1) % Self::HEIGHT))
                        .map(|(line, dest)| {
                            draw_line_wrapping(&mut dest[0], x, line)
                                | draw_line_wrapping(&mut dest[1], x, line)
                        })
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
            for plane in self.iter_enabled_planes() {
                plane.copy_within(..Self::HEIGHT as usize - amount, amount);
                for line in plane[..amount].iter_mut() {
                    *line = 0;
                }
            }
        }
        Ok(())
    }

    fn scroll_up(&mut self, amount: u4) -> Result<()> {
        let mut amount = u8::from(amount) as usize;
        if !self.hires {
            amount *= 2;
        }
        if amount > 0 {
            for plane in self.iter_enabled_planes() {
                plane.copy_within(amount.., 0);
                for line in plane[Self::HEIGHT as usize - amount..].iter_mut() {
                    *line = 0;
                }
            }
        }
        Ok(())
    }

    fn scroll_right(&mut self) -> Result<()> {
        let amount = if self.hires { 4 } else { 8 };
        for plane in self.iter_enabled_planes() {
            for line in plane.iter_mut() {
                *line >>= amount;
            }
        }
        Ok(())
    }

    fn scroll_left(&mut self) -> Result<()> {
        let amount = if self.hires { 4 } else { 8 };
        for plane in self.iter_enabled_planes() {
            for line in plane.iter_mut() {
                *line <<= amount;
            }
        }
        Ok(())
    }

    fn to_image(&self, palette: &Palette) -> RgbaImage {
        let mut image = RgbaImage::new(Self::WIDTH as u32, Self::HEIGHT as u32);
        for y in 0..Self::HEIGHT as usize {
            for (x, pixel) in combine_planes(
                self.data[0][y],
                self.data[1][y],
                self.data[2][y],
                self.data[3][y],
            )
            .into_iter()
            .enumerate()
            {
                image.put_pixel(x as u32, y as u32, palette.sixteen_color[pixel as usize]);
            }
        }
        image
    }
}

fn iter_plane_wrapping(plane: &mut [u128; 64], y: u8) -> impl Iterator<Item = &mut u128> {
    let (start, end) = plane.split_at_mut(y as usize);
    end.iter_mut().chain(start.iter_mut())
}

fn iter_plane_wrapping_pairs(plane: &mut [u128; 64], y: u8) -> impl Iterator<Item = &mut [u128]> {
    let (start, end) = plane.split_at_mut(y as usize);
    end.chunks_exact_mut(2).chain(start.chunks_exact_mut(2))
}

fn draw_line_wrapping<L>(dest: &mut u128, x: u8, line: L) -> bool
where
    u128: From<L>,
{
    draw_line(
        dest,
        x,
        line,
        |line, n| line.rotate_left(n as u32),
        |line, n| line.rotate_right(n as u32),
    )
}
