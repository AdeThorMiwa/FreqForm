use std::collections::BinaryHeap;

use crate::{
    scheduler::{command::SchedulerCommand, track::ScheduledTrack},
    track::Track,
};

pub mod command;
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

    pub fn process_command(&mut self, cmd: SchedulerCommand) {
        match cmd {
            SchedulerCommand::ScheduleTrack { track, start_frame } => {
                self.schedule(track, start_frame)
            }
            SchedulerCommand::ParamChange { target_id, change } => {
                for track in self.active_tracks.iter_mut() {
                    track.apply_param_change(&target_id, &change);
                }
            }
            SchedulerCommand::StopTrack { target_id } => {
                self.stop_track(target_id);
            }
            SchedulerCommand::RestartTrack { target_id } => {
                if let Some(track) = self
                    .active_tracks
                    .iter_mut()
                    .find(|track| track.id() == target_id)
                {
                    track.reset();
                }
            }
        }
    }

    fn schedule(&mut self, track: Box<dyn Track>, start_frame: u64) {
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

    fn stop_track(&mut self, target_id: String) {
        self.active_tracks.retain(|track| track.id() != target_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        constants::AUDIO_SAMPLE_EPSILON,
        scheduler::command::ParameterChange,
        track::{constant::ConstantTrack, gainpan::GainPanTrack, wav::WavTrack},
    };

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

    #[test]
    fn test_gain_change_applies_during_playback() {
        let gain_track =
            GainPanTrack::new("x-track", Box::new(ConstantTrack::new(1.0, 1.0)), 1.0, 0.0);
        let mut scheduler = Scheduler::new();

        scheduler.schedule(Box::new(gain_track), 0);
        scheduler.next_samples(1); // activate

        scheduler.process_command(SchedulerCommand::ParamChange {
            target_id: "x-track".to_string(),
            change: ParameterChange::SetGain(0.25),
        });

        let output = scheduler.next_samples(1);
        assert!((output[0].0 - 0.125).abs() < AUDIO_SAMPLE_EPSILON); // (1.0 * 0.25 * 0.5 pan_l)
        assert!((output[0].1 - 0.125).abs() < AUDIO_SAMPLE_EPSILON);
    }

    #[test]
    fn test_stop_track_removes_it_from_output() {
        let gpt = GainPanTrack::new("test-id", Box::new(ConstantTrack::new(0.5, 0.5)), 1.0, 0.0);
        let mut sched = Scheduler::new();
        sched.schedule(Box::new(gpt), 0);

        sched.next_samples(1); // Activate

        // Stop the track
        sched.process_command(SchedulerCommand::StopTrack {
            target_id: "test-id".into(),
        });

        let out = sched.next_samples(1);
        assert_eq!(out[0], (0.0, 0.0)); // No output = stopped
    }

    #[test]
    fn test_restart_resets_playback_position() {
        let samples = vec![(1.0, 1.0), (0.5, 0.5), (0.0, 0.0)];
        let wav = WavTrack {
            samples: samples.clone(),
            position: 0,
        };

        let gain = GainPanTrack::new("track-id", Box::new(wav), 1.0, 0.0);
        let mut sched = Scheduler::new();
        sched.schedule(Box::new(gain), 0);

        let out1 = sched.next_samples(1); // (1.0, 1.0)
        let out2 = sched.next_samples(1); // (0.5, 0.5)

        sched.process_command(SchedulerCommand::RestartTrack {
            target_id: "track-id".to_string(),
        });

        let out3 = sched.next_samples(1); // should reset to (1.0, 1.0)

        assert_eq!(out1[0], (0.5, 0.5)); // 1.0 * 1.0 * 0.5
        assert_eq!(out2[0], (0.25, 0.25));
        assert_eq!(out3[0], (0.5, 0.5)); // confirms retrigger
    }
}
