use std::collections::BinaryHeap;

use crate::{scheduler::track::ScheduledTrack, track::Track};

pub mod track;

pub struct Scheduler {
    /// a queue of future tracks
    scheduled: BinaryHeap<ScheduledTrack>,
    /// currently playing tracks
    active_tracks: Vec<Box<dyn Track>>,
    /// the current timeline position (starts at 0)
    current_frame: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            scheduled: BinaryHeap::new(),
            active_tracks: Vec::new(),
            current_frame: 0,
        }
    }

    pub fn schedule(&mut self, track: Box<dyn Track>, start_frame: u64) {
        self.scheduled.push(ScheduledTrack { track, start_frame });
    }

    pub fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        let mut buffer = vec![(0.0f32, 0.0f32); frame_size];

        while let Some(top) = self.scheduled.peek() {
            if top.start_frame <= self.current_frame {
                let ScheduledTrack { track, .. } = self.scheduled.pop().unwrap();
                self.active_tracks.push(track);
            } else {
                break;
            }
        }

        for track in self.active_tracks.iter_mut() {
            let samples = track.next_samples(frame_size);
            for (i, (l, r)) in samples.into_iter().enumerate() {
                buffer[i].0 += l;
                buffer[i].1 += r;
            }
        }

        self.current_frame += frame_size as u64;
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{constants::AUDIO_SAMPLE_EPSILON, track::constant::ConstantTrack};

    fn sum_energy(buffer: &[(f32, f32)]) -> f32 {
        buffer.iter().map(|(l, r)| l.abs() + r.abs()).sum()
    }

    #[test]
    fn test_track_scheduled_at_zero_plays_immediately() {
        let mut sched = Scheduler::new();
        sched.schedule(Box::new(ConstantTrack::new(0.1, 0.1)), 0);

        let output = sched.next_samples(4);
        assert_eq!(output.len(), 4);
        assert!(sum_energy(&output) > 0.0);
    }

    #[test]
    fn test_track_scheduled_in_future_does_not_play_early() {
        let mut sched = Scheduler::new();
        sched.schedule(Box::new(ConstantTrack::new(1.0, 1.0)), 100);

        let output = sched.next_samples(10); // still before frame 100
        assert_eq!(output.len(), 10);
        assert!(sum_energy(&output) == 0.0);

        sched.next_samples(80);
        sched.next_samples(10); // now at frame 100

        let output = sched.next_samples(1);
        assert!(sum_energy(&output) > 0.0);
    }

    #[test]
    fn test_multiple_tracks_mixed_properly() {
        let mut sched = Scheduler::new();
        sched.schedule(Box::new(ConstantTrack::new(0.3, 0.3)), 0);
        sched.schedule(Box::new(ConstantTrack::new(0.5, 0.5)), 0);

        let output = sched.next_samples(1);
        let (l, r) = output[0];
        assert!((l - 0.8).abs() < AUDIO_SAMPLE_EPSILON);
        assert!((r - 0.8).abs() < AUDIO_SAMPLE_EPSILON);
    }

    #[test]
    fn test_track_starts_midway_and_mixes_correctly() {
        let mut sched = Scheduler::new();
        sched.schedule(Box::new(ConstantTrack::new(0.5, 0.5)), 4); // start late

        let out1 = sched.next_samples(4);
        assert!(sum_energy(&out1) == 0.0); // silent

        let out2 = sched.next_samples(2);
        assert_eq!(out2.len(), 2);
        assert!((out2[0].0 - 0.5).abs() < AUDIO_SAMPLE_EPSILON);
    }
}
