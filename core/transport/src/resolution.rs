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

#[derive(Debug, Clone, Copy)]
pub enum QuantizeResolution {
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
}

impl QuantizeResolution {
    /// Returns how many grid units per beat (e.g., 4 for Sixteenth = 4 * subdivision)
    pub fn ticks_per_grid_unit(&self, ticks_per_beat: u64) -> u64 {
        match self {
            QuantizeResolution::Quarter => ticks_per_beat,
            QuantizeResolution::Eighth => ticks_per_beat / 2,
            QuantizeResolution::Sixteenth => ticks_per_beat / 4,
            QuantizeResolution::ThirtySecond => ticks_per_beat / 8,
        }
    }
}
