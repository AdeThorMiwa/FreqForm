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
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)>;
    fn apply_param_change(&mut self, _id: &str, _change: &ParameterChange) {}
}
