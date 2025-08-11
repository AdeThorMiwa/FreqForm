#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurve {
    Linear,
    EqualPower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fade {
    pub length_frames: u64,
    pub curve: FadeCurve,
}

impl Fade {
    pub const fn none() -> Self {
        Self {
            length_frames: 0,
            curve: FadeCurve::Linear,
        }
    }
}
