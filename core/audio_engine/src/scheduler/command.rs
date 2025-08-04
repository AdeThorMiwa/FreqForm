use rtrb::Consumer;
use transport::resolution::TickResolution;

use crate::track::Track;

pub enum ParameterChange {
    SetGain(f32),
    SetPan(f32),
}

pub struct LoopOptions {
    pub bar: u64,
    pub beat: u64,
    pub tick: u64,
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
