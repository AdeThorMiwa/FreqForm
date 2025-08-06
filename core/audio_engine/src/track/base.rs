use uuid::Uuid;

use crate::track::{TrackId, TrackType};

#[derive(Debug)]
pub struct BaseTrack {
    id: TrackId,
    name: String,
    track_type: TrackType,
}

impl BaseTrack {
    pub fn new(name: impl Into<String>, track_type: TrackType) -> Self {
        Self {
            id: Uuid::new_v4().into(),
            name: name.into(),
            track_type,
        }
    }

    pub fn id(&self) -> TrackId {
        self.id.to_owned()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn track_type(&self) -> TrackType {
        self.track_type
    }
}
