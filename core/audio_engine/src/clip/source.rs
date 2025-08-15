use std::fmt;

/// Represents a clip-aware audio source that supports reading at arbitrary frame offsets.
/// Implemented by WavTrack and future streamers.
pub trait ClipSource: Send + Sync + fmt::Debug {
    /// Read `frame_count` stereo frames starting from `start_frame`.
    /// Returns silence if out of bounds.
    fn read_samples(&self, start_frame: u64, frame_count: usize) -> Vec<(f32, f32)>;
}

#[cfg(test)]
#[derive(Debug)]
pub struct ConstOneSource;

#[cfg(test)]
impl ClipSource for ConstOneSource {
    fn read_samples(&self, _start_frame: u64, frame_count: usize) -> Vec<(f32, f32)> {
        vec![(1.0, 1.0); frame_count]
    }
}
