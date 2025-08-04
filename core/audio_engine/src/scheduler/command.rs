use rtrb::Consumer;
use transport::resolution::TickResolution;

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
    /// NEW: Tempo change command
    SetTempo {
        bpm: f64,
        resolution: TickResolution,
    },
}

pub type SchedulerCommandConsumer = Consumer<SchedulerCommand>;
