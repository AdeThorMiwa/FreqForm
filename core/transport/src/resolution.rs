pub enum TickResolution {
    Quarter,
    Eighth,
    Sixteenth,
}

impl TickResolution {
    pub fn divisor(&self) -> f64 {
        match self {
            TickResolution::Quarter => 1.0,
            TickResolution::Eighth => 2.0,
            TickResolution::Sixteenth => 4.0,
        }
    }
}
