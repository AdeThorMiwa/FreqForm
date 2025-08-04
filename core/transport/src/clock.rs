use crate::resolution::TickResolution;

// @todo move to core::transport
pub struct TempoClock {
    bpm: f64,
    sample_rate: f64,
    resolution: TickResolution,
    samples_per_tick: f64,
    sample_position: f64,
    tick_counter: u64,
    running: bool,
}

impl TempoClock {
    pub fn new(bpm: f64, sample_rate: f64, resolution: TickResolution) -> Self {
        let samples_per_tick = TempoClock::compute_samples_per_tick(bpm, sample_rate, &resolution);
        Self {
            bpm,
            sample_rate,
            resolution,
            samples_per_tick,
            sample_position: 0.0,
            tick_counter: 0,
            running: true,
        }
    }

    fn compute_samples_per_tick(bpm: f64, sample_rate: f64, resolution: &TickResolution) -> f64 {
        let beats_per_second = bpm / 60.0;
        let seconds_per_beat = 1.0 / beats_per_second;
        let seconds_per_tick = seconds_per_beat / resolution.divisor();
        sample_rate * seconds_per_tick
    }

    pub fn samples_per_tick(&self) -> f64 {
        self.samples_per_tick
    }

    pub fn advance_by(&mut self, samples: u64) -> bool {
        if !self.running {
            return false;
        }

        self.sample_position += samples as f64;
        let mut tick_emitted = false;

        while self.sample_position >= self.samples_per_tick {
            self.sample_position -= self.samples_per_tick;
            self.tick_counter += 1;
            tick_emitted = true;
        }

        tick_emitted
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_counter
    }

    pub fn tick_phase(&self) -> f64 {
        self.sample_position / self.samples_per_tick
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn reset(&mut self) {
        self.sample_position = 0.0;
        self.tick_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f64 = 44100.0;

    #[test]
    fn test_samples_per_tick_calculation() {
        let clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Sixteenth);
        // At 120 BPM, quarter note = 0.5 sec, 16th = 0.125 sec
        // samples_per_tick = 44100 * 0.125 = 5512.5
        assert!((clock.samples_per_tick() - 5512.5).abs() < 0.01);
    }

    #[test]
    fn test_no_tick_emitted_before_threshold() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        let tick_emitted = clock.advance_by(1000);
        assert!(!tick_emitted);
    }

    #[test]
    fn test_tick_emitted_at_threshold() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        // 120 BPM -> 0.5s per quarter -> 22050 samples
        let tick_emitted = clock.advance_by(22050);
        assert!(tick_emitted);
        assert_eq!(clock.current_tick(), 1);
    }

    #[test]
    fn test_multiple_ticks_emitted() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        // 2 ticks worth of samples
        let mut ticks = 0;
        for _ in 0..2 {
            if clock.advance_by(22050) {
                ticks += 1;
            }
        }
        assert_eq!(clock.current_tick(), 2);
        assert_eq!(ticks, 2);
    }

    #[test]
    fn test_tick_phase_accuracy() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        clock.advance_by(11025); // half a quarter note
        let phase = clock.tick_phase();
        assert!((phase - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_stop_prevents_tick() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        clock.stop();
        let tick_emitted = clock.advance_by(22050);
        assert!(!tick_emitted);
        assert_eq!(clock.current_tick(), 0);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        clock.advance_by(22050);
        clock.reset();
        assert_eq!(clock.current_tick(), 0);
        assert_eq!(clock.tick_phase(), 0.0);
    }
}
