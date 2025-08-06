use uuid::Uuid;

use crate::{
    scheduler::command::ParameterChange,
    track::{Track, TrackId},
};

#[derive(Debug)]
pub struct GainPanTrack {
    /// track id
    id: TrackId,
    inner: Box<dyn Track>,
    /// Controls signal amplitude (volume).
    /// Multiplies volume (0.0 to 1.0+)
    gain: f32,
    /// Controls left-right placement in stereo field.
    /// -1.0 = Left, 0.0 = Center, 1.0 = Right
    pan: f32,
}

impl GainPanTrack {
    pub fn new(inner: Box<dyn Track>, gain: f32, pan: f32) -> Self {
        Self {
            id: Uuid::new_v4().into(),
            inner,
            gain,
            pan,
        }
    }
}

impl Track for GainPanTrack {
    fn id(&self) -> TrackId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        "GainPan"
    }

    fn track_type(&self) -> super::TrackType {
        super::TrackType::Audio
    }

    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]) {
        // @todo review panning logic here
        let pan_l = (1.0 - self.pan.clamp(-1.0, 1.0)) * 0.5;
        let pan_r = (1.0 + self.pan.clamp(-1.0, 1.0)) * 0.5;

        self.inner.fill_next_samples(next_samples);

        for (l, r) in next_samples.iter_mut() {
            *l = *l * self.gain * pan_l;
            *r = *r * self.gain * pan_r;
        }
    }

    fn apply_param_change(&mut self, id: TrackId, change: &ParameterChange) {
        if self.id != id {
            self.inner.apply_param_change(id, change);
            return;
        }

        match change {
            ParameterChange::SetGain(val) => {
                self.gain = *val;
            }
            ParameterChange::SetPan(val) => {
                self.pan = *val;
            }
        }
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}
