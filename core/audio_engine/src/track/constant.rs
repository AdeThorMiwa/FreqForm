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

    fn next_samples(&mut self, num_frames: usize) -> Vec<(f32, f32)> {
        vec![self.sample; num_frames]
    }
}
