pub mod clip_id;
pub mod fades;
pub mod source;
use crate::clip::{
    clip_id::ClipId,
    fades::{Fade, FadeCurve},
    source::ClipSource,
};
use std::sync::Arc;
use uuid::Uuid;

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
    pub source: Arc<dyn ClipSource + Send + Sync>,

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
    pub fade_in: Fade,
    pub fade_out: Fade,
}

impl Clip {
    pub fn new_audio(
        timing: ClipTiming,
        source: Arc<dyn ClipSource + Send + Sync>,
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
            fade_in: Fade::none(),
            fade_out: Fade::none(),
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
        self.clamp_fades();
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

    pub fn set_fade_in(&mut self, length_frames: u64, curve: FadeCurve) {
        self.fade_in = Fade {
            length_frames,
            curve,
        };
        self.clamp_fades();
    }

    pub fn set_fade_out(&mut self, length_frames: u64, curve: FadeCurve) {
        self.fade_out = Fade {
            length_frames,
            curve,
        };
        self.clamp_fades();
    }

    fn clamp_fades(&mut self) {
        let len = self.timing.duration_frames;
        if self.fade_in.length_frames + self.fade_out.length_frames > len {
            // clamp proportionally (simple approach)
            let total = self.fade_in.length_frames + self.fade_out.length_frames;
            if total > 0 {
                let scale = len as f64 / total as f64;
                self.fade_in.length_frames =
                    (self.fade_in.length_frames as f64 * scale).floor() as u64;
                self.fade_out.length_frames = len - self.fade_in.length_frames;
            } else {
                self.fade_in.length_frames = 0;
                self.fade_out.length_frames = 0;
            }
        }
    }
}

#[cfg(test)]
mod clip_tests {
    use std::sync::Arc;

    use crate::{
        clip::{Clip, ClipTiming, fades::FadeCurve, source::ConstOneSource},
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

    fn make_track_with_constant_clip(
        start_frame: u64,
        duration_frames: u64,
        fade_in: Option<(u64, FadeCurve)>,
        fade_out: Option<(u64, FadeCurve)>,
    ) -> AudioTrack {
        use std::sync::Arc;
        let source = Arc::new(ConstOneSource);
        let mut clip = Clip::new_audio(
            ClipTiming {
                start_frame,
                duration_frames,
            },
            source,
            0,
            false,
            1.0,
            0.0,
        );

        if let Some((len, curve)) = fade_in {
            clip.set_fade_in(len, curve);
        }

        if let Some((len, curve)) = fade_out {
            clip.set_fade_out(len, curve);
        }

        let mut track = AudioTrack::new("FadeTest");
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

    #[test]
    fn linear_fade_in_on_constant_source() {
        let fade_len = 100u64;
        let dur = 200u64;
        let mut track =
            make_track_with_constant_clip(0, dur, Some((fade_len, FadeCurve::Linear)), None);

        let mut out = vec![(0.0f32, 0.0f32); dur as usize];
        track.fill_next_samples(&mut out);

        // Start near zero, end at ~1.0
        assert!(
            out[0].0 <= 0.01 && out[0].1 <= 0.01,
            "first sample should be ~0"
        );
        assert!(
            (out[fade_len as usize - 1].0 - 0.99).abs() < 0.05,
            "end of fade-in ~1.0"
        );
        assert!(
            (out.last().unwrap().0 - 1.0).abs() < 0.01,
            "post fade-in ~1.0"
        );
    }

    #[test]
    fn equal_power_fade_out_on_constant_source() {
        let fade_len = 100u64;
        let dur = 200u64;
        let mut track =
            make_track_with_constant_clip(0, dur, None, Some((fade_len, FadeCurve::EqualPower)));

        let mut out = vec![(0.0f32, 0.0f32); dur as usize];
        track.fill_next_samples(&mut out);

        // End near zero
        assert!(out.last().unwrap().0 <= 0.01, "last sample ~0.0");

        // Midpoint of fade-out ~ cos(pi/4) ≈ 0.707
        let fade_start = (dur - fade_len) as usize;
        let mid = fade_start + (fade_len as usize / 2);
        let v = out[mid].0; // left channel
        assert!((v - 0.707).abs() < 0.05, "mid fade-out ~0.707, got {}", v);
    }

    #[test]
    fn equal_power_crossfade_sums_to_constant() {
        use std::sync::Arc;

        // Build track with two overlapping clips
        let mut track = AudioTrack::new("Crossfade");

        // Clip A: 0..200, fade-out last 100
        let src = Arc::new(ConstOneSource);
        let mut a = Clip::new_audio(
            ClipTiming {
                start_frame: 0,
                duration_frames: 200,
            },
            src.clone(),
            0,
            false,
            1.0,
            0.0,
        );
        a.set_fade_out(100, FadeCurve::EqualPower);
        track.add_clip(a);

        // Clip B: 100..300, fade-in first 100
        let mut b = Clip::new_audio(
            ClipTiming {
                start_frame: 100,
                duration_frames: 200,
            },
            src.clone(),
            0,
            false,
            1.0,
            0.0,
        );
        b.set_fade_in(100, FadeCurve::EqualPower);
        track.add_clip(b);

        // Render 0..300
        let mut out = vec![(0.0f32, 0.0f32); 300];
        track.fill_next_samples(&mut out);

        for i in 0..100 {
            let theta = (i as f32) / 100.0 * std::f32::consts::FRAC_PI_2;
            let expected = theta.cos() + theta.sin();
            let got = out[100 + i].0;
            assert!(
                (got - expected).abs() < 0.05,
                "crossfade amp mismatch at k={}, got {}, expected {}",
                i,
                got,
                expected
            );
        }
    }
}
