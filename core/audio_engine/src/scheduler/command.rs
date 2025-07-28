use rtrb::Consumer;

use crate::track::Track;

pub enum ParameterChange {
    SetGain(f32),
    SetPan(f32),
}

// @todo change this to automation events
pub enum SchedulerCommand {
    ScheduleTrack {
        track: Box<dyn Track>,
        start_frame: u64,
    },
    ParamChange {
        target_id: String,
        change: ParameterChange,
    },
    StopTrack {
        target_id: String,
    },
    RestartTrack {
        target_id: String,
    },
}

pub type SchedulerCommandConsumer = Consumer<SchedulerCommand>;

#[cfg(test)]
mod tests {
    use crate::{scheduler::Scheduler, track::constant::ConstantTrack};

    use super::*;
    use rtrb::RingBuffer;

    #[test]
    fn test_schedule_command_adds_track_correctly() {
        let (mut producer, consumer) = RingBuffer::new(4);
        let mut scheduler = Scheduler::new(consumer);

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
        let (mut producer, consumer) = RingBuffer::new(4);
        let mut scheduler = Scheduler::new(consumer);

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
}
