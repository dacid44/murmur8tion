use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{
    asset::Asset,
    audio::{Decodable, Source},
    ecs::system::Resource,
    reflect::TypePath,
};

#[derive(Clone, Asset, TypePath, Resource)]
pub struct Chip8Audio {
    source: Arc<Mutex<Chip8Source>>,
}

impl Chip8Audio {
    pub fn new(source: Chip8Source) -> Self {
        Self {
            source: Arc::new(Mutex::new(source)),
        }
    }

    pub fn edit<T>(&self, f: impl FnOnce(&mut Chip8Source) -> T) -> T {
        let mut source = self.source.lock().unwrap();
        f(source.deref_mut())
    }
}

impl Decodable for Chip8Audio {
    type DecoderItem = f32;
    type Decoder = Box<dyn Source<Item = f32> + Send>;

    fn decoder(&self) -> Self::Decoder {
        let beeper = self.source.clone();
        Box::new(Chip8Source::default().periodic_access(
            Duration::from_secs_f64(1.0 / 1000.0),
            move |stream| {
                let beeper = beeper.lock().unwrap();
                stream.set_from(&beeper);
            },
        ))
    }
}

pub const DEFAULT_PATTERN: u128 = 0x00FF00FF00FF00FF00FF00FF00FF00FF;

pub struct Chip8Source {
    active: bool,
    rate: f64,
    pattern: u128,
    counter: f64,
}

impl Chip8Source {
    const DEFAULT_PATTERN: u128 = 0x00FF00FF00FF00FF00FF00FF00FF00FF;

    pub fn new(pitch: u8, pattern_buffer: [u8; 16]) -> Self {
        Self {
            active: false,
            rate: pitch_to_rate(pitch),
            pattern: u128::from_be_bytes(pattern_buffer),
            counter: 0.0,
        }
    }

    pub fn set_from(&mut self, other: &Self) {
        self.set_active(other.active);
        self.rate = other.rate;
        self.pattern = other.pattern;
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        if !active {
            self.counter = 0.0;
        }
    }

    pub fn set_pitch(&mut self, pitch: u8) {
        self.rate = pitch_to_rate(pitch);
    }

    pub fn set_pattern(&mut self, pattern: [u8; 16]) {
        self.pattern = u128::from_be_bytes(pattern);
    }
}

impl Default for Chip8Source {
    fn default() -> Self {
        Self {
            active: false,
            rate: 4000.0,
            pattern: Self::DEFAULT_PATTERN,
            counter: 0.0,
        }
    }
}

impl Iterator for Chip8Source {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.active {
            return Some(0.0);
        }
        self.counter += self.rate / 44100.0;
        self.counter %= 128.0;
        let index = self.counter.round() as u8;
        let sample = self.pattern & (0b1 << (127 - index)) != 0;
        Some(if sample { 1.0 } else { -1.0 })
    }
}

impl Source for Chip8Source {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn pitch_to_rate(pitch: u8) -> f64 {
    4000.0 * 2.0f64.powf((pitch as f64 - 64.0) / 48.0)
}
