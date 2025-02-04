use std::sync::{Arc, Mutex};

use bevy::{
    asset::Asset,
    audio::{Decodable, Source},
    ecs::system::Resource,
    reflect::TypePath,
};
use rodio::queue::{SourcesQueueInput, SourcesQueueOutput};

#[derive(Clone, Asset, TypePath, Resource)]
pub struct Chip8Audio {
    synth: Chip8Synth,
    queue_input: Arc<SourcesQueueInput<f32>>,
    queue_output: Arc<Mutex<Option<SourcesQueueOutput<f32>>>>,
}

impl Chip8Audio {
    pub fn new() -> Self {
        let (tx, rx) = rodio::queue::queue(true);
        Self {
            synth: Chip8Synth::new(),
            queue_input: tx,
            queue_output: Arc::new(Mutex::new(Some(rx))),
        }
    }

    pub fn render_audio(&mut self, pitch: u8, pattern: [u8; 16], timestep: f64) {
        let samples = self.synth.generate_samples(pitch, pattern, timestep);
        let source = rodio::buffer::SamplesBuffer::new(1, OUTPUT_SAMPLE_RATE, samples);
        self.queue_input.append(source);
    }

    pub fn reset(&mut self) {
        self.synth.reset()
    }
}

impl Decodable for Chip8Audio {
    type DecoderItem = f32;
    type Decoder = Box<dyn Source<Item = f32> + Send>;

    fn decoder(&self) -> Self::Decoder {
        // let beeper = self.source.clone();
        // Box::new(Chip8Source::default().periodic_access(
        //     Duration::from_secs_f64(1.0 / 1000.0),
        //     move |stream| {
        //         let beeper = beeper.lock().unwrap();
        //         stream.set_from(&beeper);
        //     },
        // ));
        let rx = self
            .queue_output
            .lock()
            .unwrap()
            .take()
            .expect("Chip8Audio decoded a second time");
        Box::new(rx.amplify(0.15))
    }
}

// pub const DEFAULT_PATTERN: [u8; 16] = 0x00FF00FF00FF00FF00FF00FF00FF00FFu128.to_be_bytes();
// pub const DEFAULT_PATTERN: [u8; 16] = 0x00FF00FF00FF00FF0F0F0F0F0F0FF0F0u128.to_be_bytes();
// pub const DEFAULT_PATTERN: [u8; 16] = 0x0F00FF0F00FF0F00FF0F00FF0F00FF0Fu128.to_be_bytes();
// pub const DEFAULT_PATTERN: [u8; 16] = 0x00FF00FF00FF00FF03F03F03F03F03F0u128.to_be_bytes();
// pub const DEFAULT_PATTERN: [u8; 16] = 0x00FF00FF00FF00FF01FC07F01FC07F00u128.to_be_bytes();
// pub const DEFAULT_PATTERN: [u8; 16] = 0x007FC07F007FC07F007FC07F007FC07Fu128.to_be_bytes();
pub const DEFAULT_PATTERN: [u8; 16] = 0x007FC01FF01FC07F007FC01FF01FC07Fu128.to_be_bytes();

const OUTPUT_SAMPLE_RATE: u32 = 44100;

#[derive(Debug, Clone)]
struct Chip8Synth {
    counter: f64,
}

impl Chip8Synth {
    fn new() -> Self {
        Self { counter: 0.0 }
    }

    fn generate_samples(&mut self, pitch: u8, pattern: [u8; 16], timestep: f64) -> Vec<f32> {
        let needed_samples = (timestep * OUTPUT_SAMPLE_RATE as f64).round() as usize;
        let rate = pitch_to_rate(pitch);
        let pattern = u128::from_be_bytes(pattern);
        let mut samples = Vec::new();
        for _ in 0..needed_samples {
            self.counter += rate / OUTPUT_SAMPLE_RATE as f64;
            self.counter %= 128.0;
            let index = self.counter.round() as u8;
            if pattern & (0b1 << (127 - index)) != 0 {
                samples.push(1.0);
            } else {
                samples.push(-1.0);
            }
        }
        samples
    }

    fn reset(&mut self) {
        self.counter = 0.0;
    }
}

fn pitch_to_rate(pitch: u8) -> f64 {
    4000.0 * 2.0f64.powf((pitch as f64 - 64.0) / 48.0)
}
