use std::any;

use crate::{scheduler::command::ParameterChange, track::audio::AudioTrack};

pub mod audio;
pub mod base;
pub mod constant;
pub mod gainpan;
pub mod midi;
pub mod sinewave;
pub mod timeline;
pub mod wav;

#[derive(Clone, PartialEq, Debug)]
pub struct TrackId(String);

impl From<uuid::Uuid> for TrackId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackType {
    Audio,
    Midi,
}

/// A track produces stereo audio frames (L, R)
pub trait Track
where
    Self: Sync + Send,
    Self: std::fmt::Debug,
    Self: any::Any,
{
    fn id(&self) -> TrackId;
    fn name(&self) -> &str;
    fn track_type(&self) -> TrackType;

    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]);
    fn apply_param_change(&mut self, _id: TrackId, _change: &ParameterChange) {}
    fn reset(&mut self) {} // Optional; for retriggerable tracks

    #[cfg(test)]
    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        let mut buf = vec![(0.0f32, 0.0f32); frame_size];
        self.fill_next_samples(&mut buf[..]);
        buf
    }
}
