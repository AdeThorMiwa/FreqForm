use crate::track::Track;

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
    fn id(&self) -> String {
        "constant-track".to_string()
    }

    fn fill_next_samples(&mut self, next_sample: &mut [(f32, f32)]) {
        for sample in next_sample.iter_mut() {
            *sample = self.sample;
        }
    }
}
