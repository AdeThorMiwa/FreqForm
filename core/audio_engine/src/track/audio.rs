use crate::track::{Track, TrackId, TrackType, base::BaseTrack};

#[derive(Debug)]
pub struct AudioTrack {
    base: BaseTrack,
}

impl AudioTrack {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            base: BaseTrack::new(name, TrackType::Audio),
        }
    }
}

impl Track for AudioTrack {
    fn id(&self) -> TrackId {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }

    fn track_type(&self) -> TrackType {
        self.base.track_type()
    }

    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]) {
        // For now, zero-fill. Will implement clip-based playback in next steps.
        for sample in next_samples.iter_mut() {
            *sample = (0.0, 0.0);
        }
    }
}

#[cfg(test)]
mod audio_track_tests {
    use super::*;

    #[test]
    fn audio_track_creation_has_valid_id_and_name() {
        let name = "My Audio Track";
        let track = AudioTrack::new(name);

        assert_eq!(track.name(), name);
        assert_eq!(track.track_type(), TrackType::Audio);
    }

    #[test]
    fn audio_track_fills_zero_samples() {
        let mut track = AudioTrack::new("Silent Track");
        let frame_size = 64;
        let samples = track.next_samples(frame_size);

        assert_eq!(samples.len(), frame_size);

        for (l, r) in samples {
            assert_eq!(l, 0.0);
            assert_eq!(r, 0.0);
        }
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
}
