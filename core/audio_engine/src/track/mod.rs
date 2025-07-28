use crate::scheduler::command::ParameterChange;

pub mod constant;
pub mod gainpan;
pub mod sinewave;
pub mod wav;

/// A track produces stereo audio frames (L, R)
pub trait Track
where
    Self: Sync + Send,
{
    fn id(&self) -> String;
    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]);
    fn apply_param_change(&mut self, _id: &str, _change: &ParameterChange) {}
    fn reset(&mut self) {} // Optional; for retriggerable tracks
    /// required for testing
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        let mut buf = vec![(0.0f32, 0.0f32); frame_size];
        self.fill_next_samples(&mut buf[..]);
        buf
    }
}
