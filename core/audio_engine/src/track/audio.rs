use uuid::Uuid;

use crate::{
    clip::Clip,
    track::{Track, TrackId, TrackType, timeline::TimelineTrack},
};

#[derive(Debug)]
pub struct AudioTrack {
    id: TrackId,
    name: String,
    timeline: TimelineTrack,
    current_frame: u64,
    playing: bool,
}

impl AudioTrack {
    pub fn new(name: impl Into<String>) -> Self {
        let id: TrackId = Uuid::new_v4().into();
        let name = name.into();
        let timeline = TimelineTrack::new(id.clone(), &name);

        Self {
            id: id.into(),
            name,
            timeline,
            current_frame: 0,
            playing: true,
        }
    }

    pub fn add_clip(&mut self, clip: Clip) {
        self.timeline.add_clip(clip);
    }

    pub fn reset_position(&mut self) {
        self.current_frame = 0;
    }

    pub fn start(&mut self) {
        self.playing = true;
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    pub fn downcast_to_audio_track(track: &mut Box<dyn Track>) -> Option<&mut AudioTrack> {
        (track as &mut dyn std::any::Any).downcast_mut::<AudioTrack>()
    }
}

impl Track for AudioTrack {
    fn id(&self) -> TrackId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn track_type(&self) -> TrackType {
        TrackType::Audio
    }

    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]) {
        if !self.playing {
            for sample in next_samples.iter_mut() {
                *sample = (0.0, 0.0);
            }
            return;
        }

        self.timeline
            .render_audio(self.current_frame, next_samples.len(), next_samples);

        self.current_frame += next_samples.len() as u64;
    }

    fn reset(&mut self) {
        self.reset_position();
        self.playing = true;
    }
}

#[cfg(test)]
mod audio_track_tests {
    use super::*;
    use crate::{clip::ClipTiming, track::wav::WavTrack};
    use std::sync::Arc;

    fn load_test_wav() -> Arc<WavTrack> {
        let wav =
            WavTrack::from_file("../../assets/wav/drum.wav").expect("Failed to load test wav");
        Arc::new(wav)
    }

    #[test]
    fn audio_track_renders_clip_output() {
        let wav = load_test_wav();

        // Clip: starts at frame 0, lasts for 512 samples
        let clip = Clip::new_audio(
            ClipTiming {
                start_frame: 0,
                duration_frames: 512,
            },
            wav.clone(),
            0,
            false,
            1.0,
            0.0,
        );

        let mut track = AudioTrack::new("Test Audio Track");
        track.add_clip(clip);

        let mut output = vec![(0.0f32, 0.0f32); 512];
        track.fill_next_samples(&mut output[..]);

        // At least some non-zero audio should be present
        let nonzero_samples = output
            .iter()
            .filter(|(l, r)| *l != 0.0 || *r != 0.0)
            .count();
        assert!(
            nonzero_samples > 0,
            "Expected non-zero samples from AudioTrack output"
        );
    }

    #[test]
    fn audio_track_silence_when_outside_clip_range() {
        let wav = load_test_wav();

        // Clip is positioned to start far in the future
        let clip = Clip::new_audio(
            ClipTiming {
                start_frame: 10_000,
                duration_frames: 256,
            },
            wav.clone(),
            0,
            false,
            1.0,
            0.0,
        );

        let mut track = AudioTrack::new("Future Clip Track");
        track.add_clip(clip);

        // Render at time 0 — clip should not be active
        let mut output = vec![(0.0f32, 0.0f32); 512];
        track.fill_next_samples(&mut output[..]);

        // All output should be silence
        for (l, r) in output {
            assert_eq!(l, 0.0);
            assert_eq!(r, 0.0);
        }
    }

    #[test]
    fn audio_track_creation_has_valid_id_and_name() {
        let name = "My Audio Track";
        let track = AudioTrack::new(name);

        assert_eq!(track.name(), name);
        assert_eq!(track.track_type(), TrackType::Audio);
    }

    #[test]
    fn audio_track_trait_object_usage() {
        let mut track: Box<dyn Track> = Box::new(AudioTrack::new("Polymorphic Track"));

        assert_eq!(track.track_type(), TrackType::Audio);
        assert_eq!(track.name(), "Polymorphic Track");

        let frame_size = 32;
        let samples = track.next_samples(frame_size);
        assert_eq!(samples.len(), frame_size);
    }

    #[test]
    fn looped_clip_repeats_samples_correctly() {
        let wav = load_test_wav();
        let loop_duration = 128;

        let clip = Clip::new_audio(
            ClipTiming {
                start_frame: 0,
                duration_frames: loop_duration,
            },
            wav.clone(),
            0,
            true,
            1.0,
            0.0,
        );

        let mut track = AudioTrack::new("Looped Clip Track");
        track.add_clip(clip);

        let mut output = vec![(0.0f32, 0.0f32); (loop_duration * 4) as usize];
        track.fill_next_samples(&mut output[..]);

        let loops = 4;
        let looped_sections: Vec<&[(f32, f32)]> = (0..loops)
            .map(|i| {
                let start = i * loop_duration;
                let end = start + loop_duration;
                &output[start as usize..end as usize]
            })
            .collect();

        for i in 1..loops {
            assert_eq!(
                looped_sections[0], looped_sections[i as usize],
                "Loop section {} does not match section 0",
                i
            );
        }
    }
}
