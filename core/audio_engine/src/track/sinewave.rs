use std::f32::consts::PI;

use uuid::Uuid;

use crate::track::{Track, TrackId};

#[derive(Clone, Debug)]
pub struct SineWaveTrack {
    id: TrackId,
    freq: f32,
    sample_rate: f32,
    phase: f32,
}

impl SineWaveTrack {
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        Self {
            id: Uuid::new_v4().into(),
            freq,
            sample_rate,
            phase: 0.0,
        }
    }
}

impl Track for SineWaveTrack {
    fn id(&self) -> TrackId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        "SineWave"
    }

    fn track_type(&self) -> super::TrackType {
        super::TrackType::Audio
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
