use std::f32::consts::PI;

use crate::track::Track;

#[derive(Clone, Copy)]
pub struct SineWaveTrack {
    freq: f32,
    sample_rate: f32,
    phase: f32,
}

impl SineWaveTrack {
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        Self {
            freq,
            sample_rate,
            phase: 0.0,
        }
    }
}

impl Track for SineWaveTrack {
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        let mut result = Vec::with_capacity(frame_size);
        let phase_increment = 2.0 * PI * self.freq / self.sample_rate;

        for _ in 0..frame_size {
            let sample = (self.phase).sin();
            result.push((sample, sample));
            self.phase += phase_increment;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }

        result
    }
}
