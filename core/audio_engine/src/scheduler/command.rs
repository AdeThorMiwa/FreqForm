use rtrb::Consumer;
use transport::resolution::TickResolution;

use crate::track::{Track, TrackId};

#[derive(Debug)]
pub enum ParameterChange {
    SetGain(f32),
    SetPan(f32),
}

#[derive(Debug)]
pub struct LoopOptions {
    pub bar: u64,
    pub beat: u64,
    pub tick: u64,
}

// @todo change this to automation events
#[derive(Debug)]
pub enum SchedulerCommand {
    ScheduleTrack {
        track: Box<dyn Track>,
        start_frame: u64,
    },
    ParamChange {
        target_id: TrackId,
        change: ParameterChange,
    },
    StopTrack {
        target_id: TrackId,
    },
    RestartTrack {
        target_id: TrackId,
    },
    /// Tempo change command
    SetTempo {
        bpm: f64,
        resolution: TickResolution,
    },
    SetLoop {
        enabled: bool,
        start: LoopOptions,
        end: LoopOptions,
    },
    Play,
    Pause,
    Stop,
}

pub type SchedulerCommandConsumer = Consumer<SchedulerCommand>;
