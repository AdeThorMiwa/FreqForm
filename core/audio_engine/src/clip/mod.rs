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
    pub kind: ClipKind,
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
            kind: ClipKind::Audio(AudioClip {
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

    pub fn trim(&mut self, new_duration: u64) {
        self.timing.duration_frames = new_duration;
    }

    pub fn move_to(&mut self, new_start_frame: u64) {
        self.timing.start_frame = new_start_frame;
    }

    pub fn slip(&mut self, delta: i64) {
        match self.kind {
            ClipKind::Audio(ref mut audio_clip) => {
                let new_offset = (audio_clip.start_offset as i64 + delta).max(0) as u64;
                audio_clip.start_offset = new_offset;
            }
        }
    }
}

#[cfg(test)]
mod clip_tests {
    use std::sync::Arc;

    use crate::{
        clip::{Clip, ClipTiming},
        track::{Track, audio::AudioTrack, wav::WavTrack},
    };

    fn load_short_test_wav() -> Arc<WavTrack> {
        let wav =
            WavTrack::from_file("./../../assets/wav/drum.wav").expect("Failed to load test wav");
        Arc::new(wav)
    }

    fn create_track_with_clip(start_frame: u64, duration_frames: u64, offset: u64) -> AudioTrack {
        let source = load_short_test_wav();

        let clip = Clip::new_audio(
            ClipTiming {
                start_frame,
                duration_frames,
            },
            source,
            offset,
            false,
            1.0,
            0.0,
        );

        let mut track = AudioTrack::new("Editing Test Track");
        track.add_clip(clip);
        track
    }

    #[test]
    fn clip_trim_reduces_output_length() {
        let mut track = create_track_with_clip(0, 256, 0);

        // Render full 256 frames first
        let mut output_1 = vec![(0.0f32, 0.0f32); 256];
        track.fill_next_samples(&mut output_1);

        // Reset + trim to 128 frames
        track.reset();
        let clip = track.timeline.clips.get_mut(0).unwrap();
        clip.trim(128);

        let mut output_2 = vec![(0.0f32, 0.0f32); 256];
        track.fill_next_samples(&mut output_2);

        // Only first 128 should be non-zero
        assert!(output_2[..128].iter().any(|(l, r)| *l != 0.0 || *r != 0.0));
        assert!(output_2[128..].iter().all(|(l, r)| *l == 0.0 && *r == 0.0));
    }

    #[test]
    fn clip_move_to_shifts_output_forward() {
        let mut track = create_track_with_clip(0, 256, 0);

        let mut output_1 = vec![(0.0f32, 0.0f32); 256];
        track.fill_next_samples(&mut output_1);

        track.reset();
        let clip = track.timeline.clips.get_mut(0).unwrap();
        clip.move_to(128); // Shift clip 128 frames forward

        let mut output_2 = vec![(0.0f32, 0.0f32); 256];
        track.fill_next_samples(&mut output_2);

        assert!(output_2[..128].iter().all(|(l, r)| *l == 0.0 && *r == 0.0));
        assert_eq!(
            output_1[..128],
            output_2[128..],
            "Expected moved clip audio to appear later"
        );
    }

    #[test]
    fn clip_slip_changes_rendered_audio_without_moving_clip() {
        let mut track = create_track_with_clip(0, 128, 0);

        let mut output_1 = vec![(0.0f32, 0.0f32); 128];
        track.fill_next_samples(&mut output_1);

        track.reset();
        let clip = track.timeline.clips.get_mut(0).unwrap();
        clip.slip(10); // Slide source 10 frames forward

        let mut output_2 = vec![(0.0f32, 0.0f32); 128];
        track.fill_next_samples(&mut output_2);

        assert_ne!(output_1, output_2, "Slip should change audio content");
    }
}
