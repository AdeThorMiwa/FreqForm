pub mod clip_id;
use crate::clip::clip_id::ClipId;
use std::{fmt, sync::Arc};
use uuid::Uuid;

/// Represents a clip-aware audio source that supports reading at arbitrary frame offsets.
/// Implemented by WavTrack and future streamers.
pub trait AudioClipSource: Send + Sync + fmt::Debug {
    /// Read `frame_count` stereo frames starting from `start_frame`.
    /// Returns silence if out of bounds.
    fn read_samples(&self, start_frame: u64, frame_count: usize) -> Vec<(f32, f32)>;
}

/// Position and length of a clip in the timeline (in samples for now)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipTiming {
    pub start_frame: u64,
    pub duration_frames: u64,
}

/// Represents a clip that plays a portion of an audio source (WAV)
#[derive(Debug)]
pub struct AudioClip {
    /// Reference to audio source (e.g. WavTrack or disk streamer)
    pub source: Arc<dyn AudioClipSource + Send + Sync>,

    /// Start offset inside the source file (for trimming)
    pub start_offset: u64,

    /// Should the clip loop during playback?
    pub looping: bool,

    /// Optional gain multiplier
    pub gain: f32,

    /// Optional stereo panning [-1.0, 1.0]
    pub pan: f32,
}

/// Supported clip content types
#[derive(Debug)]
pub enum ClipKind {
    Audio(AudioClip),
}

/// Top-level clip wrapper with metadata + content
#[derive(Debug)]
pub struct Clip {
    pub id: ClipId,
    pub timing: ClipTiming,
    pub content: ClipKind,
}

impl Clip {
    pub fn new_audio(
        timing: ClipTiming,
        source: Arc<dyn AudioClipSource + Send + Sync>,
        start_offset: u64,
        looping: bool,
        gain: f32,
        pan: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().into(),
            timing,
            content: ClipKind::Audio(AudioClip {
                source,
                start_offset,
                looping,
                gain,
                pan,
            }),
        }
    }

    pub fn is_active_at(&self, frame: u64) -> bool {
        let ClipTiming {
            start_frame,
            duration_frames,
        } = self.timing;

        frame >= start_frame && frame < (start_frame + duration_frames)
    }

    pub fn ends_at(&self) -> u64 {
        self.timing.start_frame + self.timing.duration_frames
    }
}
