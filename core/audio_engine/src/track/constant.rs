use uuid::Uuid;

use crate::track::{Track, TrackId};

#[derive(Debug)]
pub struct ConstantTrack {
    sample: (f32, f32),
}

impl ConstantTrack {
    pub fn new(left: f32, right: f32) -> Self {
        Self {
            sample: (left, right),
        }
    }
}

impl Track for ConstantTrack {
    fn id(&self) -> TrackId {
        Uuid::new_v4().into()
    }

    fn name(&self) -> &str {
        "Constant"
    }

    fn track_type(&self) -> super::TrackType {
        super::TrackType::Audio
    }

    fn fill_next_samples(&mut self, next_sample: &mut [(f32, f32)]) {
        for sample in next_sample.iter_mut() {
            *sample = self.sample;
        }
    }
}
