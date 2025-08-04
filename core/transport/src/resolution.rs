pub enum TickResolution {
    Quarter,
    Eighth,
    Sixteenth,
    PPQN(u64),
}

impl TickResolution {
    pub fn ticks_per_beat(&self) -> u64 {
        match self {
            TickResolution::Quarter => 480,
            TickResolution::Eighth => 240,
            TickResolution::Sixteenth => 120,
            TickResolution::PPQN(val) => *val,
        }
    }
}
