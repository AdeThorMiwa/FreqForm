use crate::track::Track;

/// Holds multiple tracks for mixing.
/// Produces interleaved stereo output ([f32])
pub struct Mixer {
    tracks: Vec<Box<dyn Track>>,
}

impl Mixer {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    pub fn add_track(&mut self, track: Box<dyn Track>) {
        self.tracks.push(track);
    }

    pub fn mix(&mut self, frame_size: usize) -> Vec<f32> {
        let mut mix_buffer = vec![(0.0f32, 0.0f32); frame_size];

        for track in self.tracks.iter_mut() {
            let samples = track.next_samples(frame_size);
            for (i, (l, r)) in samples.iter().enumerate() {
                mix_buffer[i].0 += l;
                mix_buffer[i].1 += r;
            }
        }

        let mut interleave_buffer = Vec::with_capacity(frame_size * 2);
        for (l, r) in mix_buffer {
            interleave_buffer.push(l);
            interleave_buffer.push(r);
        }

        interleave_buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::{constant::ConstantTrack, gainpan::GainPanTrack};

    #[test]
    fn test_gain_one_pan_center_should_preserve_sample() {
        let track = ConstantTrack::new(1.0, 1.0);
        let mut wrapped = GainPanTrack::new("x-track", Box::new(track), 1.0, 0.0);

        let samples = wrapped.next_samples(1);
        assert_eq!(samples[0].0, 0.5); // (1.0 * 1.0 * 0.5)
        assert_eq!(samples[0].1, 0.5);
    }

    #[test]
    fn test_gain_half_pan_center_should_reduce_volume_evenly() {
        let track = ConstantTrack::new(1.0, 1.0);
        let mut wrapped = GainPanTrack::new("x-track", Box::new(track), 0.5, 0.0);

        let samples = wrapped.next_samples(1);
        assert_eq!(samples[0].0, 0.25); // (1.0 * 0.5 * 0.5)
        assert_eq!(samples[0].1, 0.25);
    }

    #[test]
    fn test_pan_left_should_output_left_only() {
        let track = ConstantTrack::new(1.0, 1.0);
        let mut wrapped = GainPanTrack::new("x-track", Box::new(track), 1.0, -1.0);

        let samples = wrapped.next_samples(1);
        assert_eq!(samples[0].0, 1.0); // Left channel full
        assert_eq!(samples[0].1, 0.0); // Right channel muted
    }

    #[test]
    fn test_pan_right_should_output_right_only() {
        let track = ConstantTrack::new(1.0, 1.0);
        let mut wrapped = GainPanTrack::new("x-track", Box::new(track), 1.0, 1.0);

        let samples = wrapped.next_samples(1);
        assert_eq!(samples[0].0, 0.0); // Left muted
        assert_eq!(samples[0].1, 1.0); // Right full
    }

    #[test]
    fn test_mixer_with_two_tracks_should_sum_samples() {
        let mut mixer = Mixer::new();

        // Each track produces (0.2, 0.4)
        let t1 = ConstantTrack::new(0.2, 0.4);
        let t2 = ConstantTrack::new(0.3, 0.6);

        mixer.add_track(Box::new(t1));
        mixer.add_track(Box::new(t2));

        let output = mixer.mix(1);
        assert_eq!(output, vec![0.5, 1.0]); // L = 0.2 + 0.3, R = 0.4 + 0.6
    }

    #[test]
    fn test_mixer_with_no_tracks_should_output_silence() {
        let mut mixer = Mixer::new();
        let output = mixer.mix(2);
        assert_eq!(output, vec![0.0, 0.0, 0.0, 0.0]);
    }
}
