pub mod clip_id;
use std::sync::Arc;
use uuid::Uuid;

use crate::{clip::clip_id::ClipId, device_manager::AudioSource};

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
    pub source: Arc<dyn AudioSource + Send + Sync>,

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
pub enum ClipContent {
    Audio(AudioClip),
}

/// Top-level clip wrapper with metadata + content
#[derive(Debug)]
pub struct Clip {
    pub id: ClipId,
    pub timing: ClipTiming,
    pub content: ClipContent,
}

impl Clip {
    pub fn new_audio(
        timing: ClipTiming,
        source: Arc<dyn AudioSource + Send + Sync>,
        start_offset: u64,
        looping: bool,
        gain: f32,
        pan: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().into(),
            timing,
            content: ClipContent::Audio(AudioClip {
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
