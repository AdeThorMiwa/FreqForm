use std::collections::BinaryHeap;

use cpal::Sample;
use transport::{clock::TempoClock, timeline::TimelinePosition, transport::TransportState};

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

pub struct LoopPoints {
    pub start_bar: u64,
    pub start_beat: u64,
    pub start_tick: u64,
    pub end_bar: u64,
    pub end_beat: u64,
    pub end_tick: u64,
}

pub struct Scheduler {
    /// a queue of future tracks
    scheduled: BinaryHeap<ScheduledTrack>,
    /// currently playing tracks
    active_tracks: Vec<Box<dyn Track>>,
    /// the current timeline position (starts at 0)
    current_frame: u64,
    automation_events: SchedulerCommandConsumer,
    /// Global tempo clock
    tempo_clock: TempoClock,
    /// Sample rate, injected at runtime
    sample_rate: f64,

    looping_enabled: bool,
    loop_points: Option<LoopPoints>,
    loop_start_frame: u64,
    loop_end_frame: u64,

    transport_state: TransportState,
}

impl Scheduler {
    pub fn new(consumer: SchedulerCommandConsumer, tempo_clock: TempoClock) -> Self {
        Self {
            scheduled: BinaryHeap::new(),
            active_tracks: Vec::new(),
            current_frame: 0,
            automation_events: consumer,
            sample_rate: tempo_clock.sample_rate(),
            tempo_clock,
            looping_enabled: false,
            loop_points: None,
            loop_start_frame: 0,
            loop_end_frame: 0,
            transport_state: TransportState::Stopped,
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
            SchedulerCommand::SetLoop {
                enabled,
                start,
                end,
            } => {
                self.looping_enabled = enabled;

                if enabled {
                    let loop_points = LoopPoints {
                        start_bar: start.bar,
                        start_beat: start.beat,
                        start_tick: start.tick,
                        end_bar: end.bar,
                        end_beat: end.beat,
                        end_tick: end.tick,
                    };

                    let start_ticks = self.bbt_to_tick_count(&loop_points, true);
                    let end_ticks = self.bbt_to_tick_count(&loop_points, false);

                    let start_frame =
                        (start_ticks as f64 * self.tempo_clock.samples_per_tick()).round() as u64;
                    let end_frame =
                        (end_ticks as f64 * self.tempo_clock.samples_per_tick()).round() as u64;

                    self.loop_points = Some(loop_points);
                    self.loop_start_frame = start_frame;
                    self.loop_end_frame = end_frame;
                } else {
                    self.loop_points = None;
                }
            }
            SchedulerCommand::Play => {
                self.transport_state = TransportState::Playing;
                self.tempo_clock.start();
            }
            SchedulerCommand::Pause => {
                self.transport_state = TransportState::Paused;
            }
            SchedulerCommand::Stop => {
                self.transport_state = TransportState::Stopped;
                self.current_frame = 0;
                self.tempo_clock.reset();
                self.active_tracks.clear(); // stop playback
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

        if self.transport_state != TransportState::Playing {
            return buffer;
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

        // Loop wrap logic
        if self.looping_enabled && self.current_frame >= self.loop_end_frame {
            self.current_frame = self.loop_start_frame;
            self.tempo_clock.reset();
            self.tempo_clock.advance_by(self.current_frame); // Sync tick position to loop start
        }

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

    fn bbt_to_tick_count(&self, loop_points: &LoopPoints, start: bool) -> u64 {
        let (bar, beat, tick) = if start {
            (
                loop_points.start_bar,
                loop_points.start_beat,
                loop_points.start_tick,
            )
        } else {
            (
                loop_points.end_bar,
                loop_points.end_beat,
                loop_points.end_tick,
            )
        };

        let ticks_per_beat = self.tempo_clock.ticks_per_beat;
        let beats_per_bar = self.tempo_clock.time_signature.beats_per_bar;

        let total_ticks = ((bar - 1) * beats_per_bar * ticks_per_beat)
            + ((beat - 1) * ticks_per_beat)
            + (tick - 1);

        total_ticks
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

    pub fn get_timeline_position(&self) -> TimelinePosition {
        let (bar, beat, tick_within_beat) = self.tempo_clock.bar_beat_tick();
        let tick = self.current_tick();

        TimelinePosition {
            bar,
            beat,
            tick,
            current_frame: self.current_frame,
            tick_within_beat,
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

//@todo move this guys to somewhere else, anywhere.. just get them tf out this file
#[cfg(test)]
mod test_util {
    use crate::scheduler::{Scheduler, command::SchedulerCommand};
    use rtrb::{Producer, RingBuffer};
    use transport::clock::TempoClock;

    pub fn create_scheduler_with_channel() -> (Scheduler, Producer<SchedulerCommand>) {
        let (producer, consumer) = RingBuffer::new(32);
        let tempo_clock = TempoClock::new(
            120.0,
            44100.0,
            transport::resolution::TickResolution::Sixteenth,
        );
        let scheduler = Scheduler::new(consumer, tempo_clock);
        (scheduler, producer)
    }
}

#[cfg(test)]
mod tests {
    use rtrb::RingBuffer;
    use transport::resolution::TickResolution;

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
        let (mut sched, _) = test_util::create_scheduler_with_channel();
        sched.schedule(Box::new(ConstantTrack::new(0.1, 0.1)), 0);
        sched.process_command(SchedulerCommand::Play);

        let output = sched.next_samples(4);
        assert_eq!(output.len(), 4);
        assert!(sum_energy(&output) > 0.0);
    }

    #[test]
    fn test_track_scheduled_in_future_does_not_play_early() {
        let (mut sched, _) = test_util::create_scheduler_with_channel();
        sched.schedule(Box::new(ConstantTrack::new(1.0, 1.0)), 100);
        sched.process_command(SchedulerCommand::Play);

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
        let (mut sched, _) = test_util::create_scheduler_with_channel();
        sched.schedule(Box::new(ConstantTrack::new(0.3, 0.3)), 0);
        sched.schedule(Box::new(ConstantTrack::new(0.5, 0.5)), 0);
        sched.process_command(SchedulerCommand::Play);

        let output = sched.next_samples(1);
        let (l, r) = output[0];
        assert!((l - 0.8).abs() < AUDIO_SAMPLE_EPSILON);
        assert!((r - 0.8).abs() < AUDIO_SAMPLE_EPSILON);
    }

    #[test]
    fn test_track_starts_midway_and_mixes_correctly() {
        let (mut sched, _) = test_util::create_scheduler_with_channel();
        sched.process_command(SchedulerCommand::Play);
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
        let (mut scheduler, _) = test_util::create_scheduler_with_channel();

        scheduler.schedule(Box::new(gain_track), 0);
        scheduler.process_command(SchedulerCommand::Play);
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
        let (mut sched, _) = test_util::create_scheduler_with_channel();
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
        let (mut sched, _) = test_util::create_scheduler_with_channel();
        sched.schedule(Box::new(gain), 0);
        sched.process_command(SchedulerCommand::Play);

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
        let (mut scheduler, mut producer) = test_util::create_scheduler_with_channel();
        scheduler.process_command(SchedulerCommand::Play);

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
        let (mut scheduler, mut producer) = test_util::create_scheduler_with_channel();
        scheduler.process_command(SchedulerCommand::Play);

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
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();
        prod.push(SchedulerCommand::Play).unwrap();

        // 16th note = 5512.5 samples at 120 BPM
        scheduler.next_samples(5513); // cross one 16th note
        assert_eq!(scheduler.current_tick(), 30);
    }

    // #[test]
    // fn test_tick_phase_after_partial_advancement() {
    //     let (mut scheduler, _) = test_util::create_scheduler_with_channel();
    //     scheduler.process_command(SchedulerCommand::Play);

    //     scheduler.next_samples(2756); // half of 5512.5
    //     let phase = scheduler.tick_phase();
    //     assert!((phase - 0.5).abs() < 0.05);
    // }

    #[test]
    fn test_set_tempo_resets_clock() {
        let (mut scheduler, mut producer) = test_util::create_scheduler_with_channel();
        scheduler.process_command(SchedulerCommand::Play);

        scheduler.next_samples(5513); // advance by one 16th note
        assert_eq!(scheduler.current_tick(), 30);

        producer
            .push(SchedulerCommand::SetTempo {
                bpm: 60.0,
                resolution: TickResolution::Quarter,
            })
            .unwrap();

        scheduler.next_samples(100); // process command
        assert_eq!(scheduler.current_tick(), 1);

        // At 60 BPM, quarter note = 44100 samples
        scheduler.next_samples(44100);
        assert_eq!(scheduler.current_tick(), 481);
    }
}

#[cfg(test)]
mod scheduler_loop_tests {
    use crate::scheduler::command::LoopOptions;

    use super::*;

    #[test]
    fn test_loop_point_setup_and_conversion() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::SetLoop {
            enabled: true,
            start: LoopOptions {
                bar: 1,
                beat: 1,
                tick: 1,
            },
            end: LoopOptions {
                bar: 2,
                beat: 1,
                tick: 1,
            },
        })
        .unwrap();

        scheduler.next_samples(1); // process command

        assert!(scheduler.looping_enabled);
        assert!(scheduler.loop_points.is_some());

        // 4/4 time, 120 ticks/beat: 480 ticks/bar
        // start_tick = 0, end_tick = 480
        let expected_start_frame = 0;
        let expected_end_frame = (480.0 * scheduler.tempo_clock.samples_per_tick()).round() as u64;

        assert_eq!(scheduler.loop_start_frame, expected_start_frame);
        assert_eq!(scheduler.loop_end_frame, expected_end_frame);
    }

    #[test]
    fn test_looping_wraps_current_frame() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::SetLoop {
            enabled: true,
            start: LoopOptions {
                bar: 1,
                beat: 1,
                tick: 1,
            },
            end: LoopOptions {
                bar: 1,
                beat: 2,
                tick: 1,
            },
        })
        .unwrap();

        scheduler.next_samples(1); // process command

        let loop_end = scheduler.loop_end_frame;

        // Advance just past end frame
        scheduler.next_samples(loop_end as usize + 1);

        // Should wrap to start frame (0)
        assert_eq!(scheduler.current_frame, scheduler.loop_start_frame);
    }

    #[test]
    fn test_tick_sync_after_loop_wrap() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::SetLoop {
            enabled: true,
            start: LoopOptions {
                bar: 1,
                beat: 1,
                tick: 1,
            },
            end: LoopOptions {
                bar: 1,
                beat: 2,
                tick: 1,
            },
        })
        .unwrap();

        scheduler.next_samples(1); // process command

        let loop_end = scheduler.loop_end_frame;

        scheduler.next_samples(loop_end as usize + 1);

        // After wrap, tick should be synced to current_frame
        let expected_tick_float =
            scheduler.current_frame as f64 / scheduler.tempo_clock.samples_per_tick();
        let expected_tick = expected_tick_float.floor() as u64;

        assert_eq!(scheduler.current_tick(), expected_tick);
    }

    #[test]
    fn test_looping_disabled_no_wrap() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();
        prod.push(SchedulerCommand::Play).unwrap();

        prod.push(SchedulerCommand::SetLoop {
            enabled: true,
            start: LoopOptions {
                bar: 1,
                beat: 1,
                tick: 1,
            },
            end: LoopOptions {
                bar: 1,
                beat: 2,
                tick: 1,
            },
        })
        .unwrap();

        scheduler.next_samples(1); // process command

        prod.push(SchedulerCommand::SetLoop {
            enabled: false,
            start: LoopOptions {
                bar: 1,
                beat: 1,
                tick: 1,
            },
            end: LoopOptions {
                bar: 1,
                beat: 2,
                tick: 1,
            },
        })
        .unwrap();

        scheduler.next_samples(1); // disable command

        let loop_end = scheduler.loop_end_frame;
        scheduler.next_samples(loop_end as usize + 1);

        println!(
            "loop: {} {}",
            scheduler.current_frame, scheduler.loop_end_frame
        );

        // Should not wrap
        assert!(scheduler.current_frame > scheduler.loop_end_frame);
    }
}

#[cfg(test)]
mod scheduler_transport_tests {
    use crate::track::constant::ConstantTrack;

    use super::*;

    #[test]
    fn test_initial_state_stopped() {
        let (mut scheduler, _) = test_util::create_scheduler_with_channel();

        let output = scheduler.next_samples(512);

        // Should be silence
        assert!(output.iter().all(|(l, r)| *l == 0.0 && *r == 0.0));
        assert_eq!(scheduler.current_frame, 0);
        assert_eq!(scheduler.current_tick(), 0);
    }

    #[test]
    fn test_play_advances_frame_and_tick() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::ScheduleTrack {
            track: Box::new(ConstantTrack::new(0.2, 0.2)),
            start_frame: 0,
        })
        .unwrap();

        prod.push(SchedulerCommand::Play).unwrap();
        scheduler.next_samples(512); // process Play command

        let output = scheduler.next_samples(512);

        assert!(output.iter().any(|(l, r)| *l != 0.0 || *r != 0.0));
        assert!(scheduler.current_frame > 0);

        assert!(scheduler.current_tick() > 0);
    }

    #[test]
    fn test_pause_halts_time_and_silences_output() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::Play).unwrap();
        scheduler.next_samples(512); // process Play
        scheduler.next_samples(512); // advance

        let frame_after_play = scheduler.current_frame;
        let tick_after_play = scheduler.current_tick();

        prod.push(SchedulerCommand::Pause).unwrap();
        scheduler.next_samples(512); // process Pause

        let output = scheduler.next_samples(512);

        assert!(output.iter().all(|(l, r)| *l == 0.0 && *r == 0.0));
        assert_eq!(scheduler.current_frame, frame_after_play);
        assert_eq!(scheduler.current_tick(), tick_after_play);
    }

    #[test]
    fn test_stop_resets_frame_and_tick() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::Play).unwrap();
        scheduler.next_samples(512); // process Play
        scheduler.next_samples(512); // advance

        prod.push(SchedulerCommand::Stop).unwrap();
        scheduler.next_samples(512); // process Stop

        assert_eq!(scheduler.current_frame, 0);
        assert_eq!(scheduler.current_tick(), 0);
        let output = scheduler.next_samples(512);
        assert!(output.iter().all(|(l, r)| *l == 0.0 && *r == 0.0));
    }

    #[test]
    fn test_resume_after_pause() {
        let (mut scheduler, mut prod) = test_util::create_scheduler_with_channel();

        prod.push(SchedulerCommand::Play).unwrap();
        scheduler.next_samples(512); // process Play
        scheduler.next_samples(512); // advance

        let frame_after_play = scheduler.current_frame;
        let tick_after_play = scheduler.current_tick();

        prod.push(SchedulerCommand::Pause).unwrap();
        scheduler.next_samples(512); // process Pause
        scheduler.next_samples(512); // hold

        prod.push(SchedulerCommand::Play).unwrap();
        scheduler.next_samples(512); // process Play
        scheduler.next_samples(512); // advance again

        assert!(scheduler.current_frame > frame_after_play);
        assert!(scheduler.current_tick() > tick_after_play);
    }
}
