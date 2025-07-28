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
    fn id(&self) -> String {
        "sine-wave-track".to_owned()
    }

    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]) {
        let phase_increment = 2.0 * PI * self.freq / self.sample_rate;

        for (l, r) in next_samples {
            let sample = (self.phase).sin();
            *l = sample;
            *r = sample;
            self.phase += phase_increment;
            if self.phase >= 2.0 * PI {
                self.phase -= 2.0 * PI;
            }
        }
    }
}
