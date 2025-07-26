use crate::track::Track;

pub struct GainPanTrack {
    inner: Box<dyn Track>,
    /// Controls signal amplitude (volume).
    /// Multiplies volume (0.0 to 1.0+)
    gain: f32,
    /// Controls left-right placement in stereo field.
    /// -1.0 = Left, 0.0 = Center, 1.0 = Right
    pan: f32,
}

impl GainPanTrack {
    pub fn new(inner: Box<dyn Track>, gain: f32, pan: f32) -> Self {
        Self { inner, gain, pan }
    }
}

impl Track for GainPanTrack {
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        let pan_l = (1.0 - self.pan.clamp(-1.0, 1.0)) * 0.5;
        let pan_r = (1.0 + self.pan.clamp(-1.0, 1.0)) * 0.5;

        let samples = self.inner.next_samples(frame_size).into_iter();

        let apply_gain_and_pan = move |(l, r)| {
            let l = l * self.gain * pan_l;
            let r = r * self.gain * pan_r;
            (l, r)
        };

        samples.map(apply_gain_and_pan).collect()
    }
}
