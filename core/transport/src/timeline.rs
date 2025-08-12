#[derive(Debug, Clone, Copy)]
pub struct TimelinePosition {
    pub current_frame: u64,
    pub bar: u64,
    pub beat: u64,
    pub tick: u64,
    pub tick_within_beat: u64,
}
