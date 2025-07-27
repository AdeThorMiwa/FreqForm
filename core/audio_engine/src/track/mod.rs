pub mod gainpan;
pub mod sinewave;
pub mod wav;

/// A track produces stereo audio frames (L, R)
pub trait Track
where
    Self: Sync + Send,
{
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)>;
}
