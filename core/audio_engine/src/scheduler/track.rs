use crate::track::Track;

#[derive(Debug)]
pub struct ScheduledTrack {
    /// Track to be scheduled
    pub track: Box<dyn Track>,
    /// the frame to start playing track
    pub start_frame: u64,
}

impl PartialEq for ScheduledTrack {
    fn eq(&self, other: &Self) -> bool {
        self.start_frame == other.start_frame
    }
}

impl Eq for ScheduledTrack {}

impl PartialOrd for ScheduledTrack {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.start_frame.cmp(&self.start_frame))
    }
}

impl Ord for ScheduledTrack {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.start_frame.cmp(&self.start_frame)
    }
}
