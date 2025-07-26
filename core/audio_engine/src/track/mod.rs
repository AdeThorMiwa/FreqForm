pub mod gainpan;

/// A track produces stereo audio frames (L, R)
pub trait Track {
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)>;
}
