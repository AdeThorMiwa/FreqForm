use std::collections::BinaryHeap;

use cpal::Sample;
use transport::{clock::TempoClock, resolution::TickResolution};

use crate::{
    device_manager::{AudioSource, AudioSourceBufferKind},
    scheduler::{
        command::{SchedulerCommand, SchedulerCommandConsumer},
        track::ScheduledTrack,
    },
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
    automation_events: SchedulerCommandConsumer,
    /// NEW: Global tempo clock
    tempo_clock: TempoClock,
    /// NEW: Sample rate, injected at runtime
    sample_rate: f64,
}

impl Scheduler {
    pub fn new(consumer: SchedulerCommandConsumer, sample_rate: f64) -> Self {
        let bpm = 120.0;
        let resolution = TickResolution::Sixteenth;
        let tempo_clock = TempoClock::new(bpm, sample_rate, resolution);

        Self {
            scheduled: BinaryHeap::new(),
            active_tracks: Vec::new(),
            current_frame: 0,
            automation_events: consumer,
            tempo_clock,
            sample_rate,
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
            SchedulerCommand::SetTempo { bpm, resolution } => {
                self.tempo_clock = TempoClock::new(bpm, self.sample_rate, resolution);
            }
        }
    }

    fn schedule(&mut self, track: Box<dyn Track>, start_frame: u64) {
        self.scheduled.push(ScheduledTrack { track, start_frame });
    }

    pub fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        // @audit allocation here, needs review
        let mut buffer = vec![(0.0f32, 0.0f32); frame_size];

        while let Ok(cmd) = self.automation_events.pop() {
            self.process_command(cmd);
        }

        while let Some(top) = self.scheduled.peek() {
            if top.start_frame <= self.current_frame {
                let ScheduledTrack { track, .. } = self.scheduled.pop().unwrap();
                // @audit possible allocation here
                self.active_tracks.push(track);
            } else {
                break;
            }
        }

        // @audit allocation here, needs review
        let mut tmp_buffer = vec![(0.0f32, 0.0f32); frame_size];
        for track in self.active_tracks.iter_mut() {
            track.fill_next_samples(&mut tmp_buffer[..]);
            for (i, (l, r)) in tmp_buffer.iter().enumerate() {
                buffer[i].0 += l;
                buffer[i].1 += r;
            }
        }

        // Advance the tempo clock by the number of samples processed
        self.tempo_clock.advance_by(frame_size as u64);

        self.current_frame += frame_size as u64;
        buffer
    }

    fn stop_track(&mut self, target_id: String) {
        self.active_tracks.retain(|track| track.id() != target_id);
    }

    pub fn current_tick(&self) -> u64 {
        self.tempo_clock.current_tick()
    }

    pub fn tick_phase(&self) -> f64 {
        self.tempo_clock.tick_phase()
    }

    fn fill_sample<T>(&self, data: &mut [T], samples: &[(f32, f32)])
    where
        T: cpal::FromSample<f32>,
    {
        for (i, sample) in data.iter_mut().enumerate() {
            let channel = i % 2; // wrap 
            let raw_sample = if channel == 0 {
                samples[i / 2].0
            } else {
                samples[i / 2].1
            };
            *sample = raw_sample.to_sample::<T>();
        }
    }
}

impl AudioSource for Scheduler {
    fn fill_buffer(&mut self, buffer: AudioSourceBufferKind<'_>, frame_size: usize) {
        let stereo_samples = self.next_samples(frame_size);

        match buffer {
            AudioSourceBufferKind::F32(data) => {
                self.fill_sample(data, &stereo_samples[..]);
            }
            AudioSourceBufferKind::I16(data) => {
                self.fill_sample(data, &stereo_samples[..]);
            }
            AudioSourceBufferKind::U16(data) => {
                self.fill_sample(data, &stereo_samples[..]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rtrb::{Producer, RingBuffer};

    use super::*;
    use crate::{
        constants::AUDIO_SAMPLE_EPSILON,
        scheduler::command::ParameterChange,
        track::{constant::ConstantTrack, gainpan::GainPanTrack, wav::WavTrack},
    };

    fn create_scheduler_with_channel() -> (Scheduler, Producer<SchedulerCommand>) {
        let (producer, consumer) = RingBuffer::new(32);
        let scheduler = Scheduler::new(consumer, 44100.0);
        (scheduler, producer)
    }

    fn sum_energy(buffer: &[(f32, f32)]) -> f32 {
        buffer.iter().map(|(l, r)| l.abs() + r.abs()).sum()
    }

    #[test]
    fn test_track_scheduled_at_zero_plays_immediately() {
        let (mut sched, _) = create_scheduler_with_channel();
        sched.schedule(Box::new(ConstantTrack::new(0.1, 0.1)), 0);

        let output = sched.next_samples(4);
        assert_eq!(output.len(), 4);
        assert!(sum_energy(&output) > 0.0);
    }

    #[test]
    fn test_track_scheduled_in_future_does_not_play_early() {
        let (mut sched, _) = create_scheduler_with_channel();
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
        let (mut sched, _) = create_scheduler_with_channel();
        sched.schedule(Box::new(ConstantTrack::new(0.3, 0.3)), 0);
        sched.schedule(Box::new(ConstantTrack::new(0.5, 0.5)), 0);

        let output = sched.next_samples(1);
        let (l, r) = output[0];
        assert!((l - 0.8).abs() < AUDIO_SAMPLE_EPSILON);
        assert!((r - 0.8).abs() < AUDIO_SAMPLE_EPSILON);
    }

    #[test]
    fn test_track_starts_midway_and_mixes_correctly() {
        let (mut sched, _) = create_scheduler_with_channel();
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
        let (mut scheduler, _) = create_scheduler_with_channel();

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
        let (mut sched, _) = create_scheduler_with_channel();
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
        let (mut sched, _) = create_scheduler_with_channel();
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

    #[test]
    fn test_schedule_command_adds_track_correctly() {
        let (mut scheduler, mut producer) = create_scheduler_with_channel();

        // Push command to ring
        let success = producer.push(SchedulerCommand::ScheduleTrack {
            track: Box::new(ConstantTrack::new(0.4, 0.4)),
            start_frame: 0,
        });
        assert!(success.is_ok(), "Should be able to enqueue command");

        // Should now have one active track
        let output = scheduler.next_samples(2);
        assert_eq!(output.len(), 2);
        assert!((output[0].0 - 0.4).abs() < 1e-6);
    }

    #[test]
    fn test_scheduled_track_via_command_plays_at_correct_time() {
        let (mut scheduler, mut producer) = create_scheduler_with_channel();

        // Send command for track to start at frame 3
        producer
            .push(SchedulerCommand::ScheduleTrack {
                track: Box::new(ConstantTrack::new(0.2, 0.2)),
                start_frame: 3,
            })
            .expect("Failed to enqueue");

        // Advance scheduler past frame 3
        let silent = scheduler.next_samples(3);
        assert!(silent.iter().all(|&(l, r)| (l + r).abs() < 1e-6));

        let active = scheduler.next_samples(1);
        assert!((active[0].0 - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_ring_buffer_drops_when_full() {
        let (mut prod, mut cons) = RingBuffer::new(1);

        // Fill it
        prod.push(SchedulerCommand::ScheduleTrack {
            track: Box::new(ConstantTrack::new(0.0, 0.0)),
            start_frame: 0,
        })
        .unwrap();

        // Next push should fail
        let result = prod.push(SchedulerCommand::ScheduleTrack {
            track: Box::new(ConstantTrack::new(0.1, 0.1)),
            start_frame: 0,
        });

        assert!(
            result.is_err(),
            "Second command should fail due to full ring buffer"
        );

        // Consume one and push again
        let _ = cons.pop();
        assert!(
            prod.push(SchedulerCommand::ScheduleTrack {
                track: Box::new(ConstantTrack::new(0.2, 0.2)),
                start_frame: 0,
            })
            .is_ok()
        );
    }

    #[test]
    fn test_clock_advances_with_next_samples() {
        let (mut scheduler, _) = create_scheduler_with_channel();

        // 16th note = 5512.5 samples at 120 BPM
        scheduler.next_samples(5513); // cross one 16th note
        assert_eq!(scheduler.current_tick(), 1);
    }

    #[test]
    fn test_tick_phase_after_partial_advancement() {
        let (mut scheduler, _) = create_scheduler_with_channel();

        scheduler.next_samples(2756); // half of 5512.5
        let phase = scheduler.tick_phase();
        assert!((phase - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_set_tempo_resets_clock() {
        let (mut scheduler, mut producer) = create_scheduler_with_channel();

        scheduler.next_samples(5513); // advance by one 16th note
        assert_eq!(scheduler.current_tick(), 1);

        producer
            .push(SchedulerCommand::SetTempo {
                bpm: 60.0,
                resolution: TickResolution::Quarter,
            })
            .unwrap();

        scheduler.next_samples(100); // process command
        assert_eq!(scheduler.current_tick(), 0);

        // At 60 BPM, quarter note = 44100 samples
        scheduler.next_samples(44100);
        assert_eq!(scheduler.current_tick(), 1);
    }
}
