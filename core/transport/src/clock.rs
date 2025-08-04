use crate::resolution::TickResolution;

#[derive(Debug, Clone, Copy)]
pub struct TimeSignature {
    pub beats_per_bar: u64, // numerator (e.g., 4 in 4/4)
    pub beat_unit: u64,     // denominator (e.g., 4 in 4/4)
}

// @todo move to core::transport
pub struct TempoClock {
    bpm: f64,
    sample_rate: f64,
    resolution: TickResolution,
    samples_per_tick: f64,
    sample_position: f64,
    tick_counter: u64,
    running: bool,
    pub time_signature: TimeSignature,
    pub ticks_per_beat: u64,
}

impl TempoClock {
    pub fn new(bpm: f64, sample_rate: f64, resolution: TickResolution) -> Self {
        let time_signature = TimeSignature {
            beats_per_bar: 4,
            beat_unit: 4,
        };
        Self::with_signature(bpm, sample_rate, resolution, time_signature, 4)
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

    fn with_signature(
        bpm: f64,
        sample_rate: f64,
        resolution: TickResolution,
        time_signature: TimeSignature,
        ticks_per_beat: u64,
    ) -> Self {
        let samples_per_tick = TempoClock::compute_samples_per_tick(bpm, sample_rate, &resolution);
        Self {
            bpm,
            sample_rate,
            resolution,
            samples_per_tick,
            sample_position: 0.0,
            tick_counter: 0,
            running: true,
            time_signature,
            ticks_per_beat,
        }
    }

    pub fn bar_beat_tick(&self) -> (u64, u64, u64) {
        let ticks_per_bar = self.ticks_per_beat * self.time_signature.beats_per_bar;

        let bar = self.tick_counter / ticks_per_bar + 1;
        let ticks_into_bar = self.tick_counter % ticks_per_bar;

        let beat = ticks_into_bar / self.ticks_per_beat + 1;
        let tick = ticks_into_bar % self.ticks_per_beat + 1;

        (bar, beat, tick)
    }
}

#[cfg(test)]
impl TempoClock {
    pub fn mock_set_tick_counter(&mut self, value: u64) {
        self.tick_counter = value;
    }
}

#[cfg(test)]
mod temp_clock_base_tests {
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

#[cfg(test)]
mod bar_beat_tick_tests {
    use super::*;

    fn create_clock(
        bpm: f64,
        sample_rate: f64,
        beats_per_bar: u64,
        beat_unit: u64,
        ticks_per_beat: u64,
    ) -> TempoClock {
        TempoClock::with_signature(
            bpm,
            sample_rate,
            TickResolution::Sixteenth,
            TimeSignature {
                beats_per_bar,
                beat_unit,
            },
            ticks_per_beat,
        )
    }

    #[test]
    fn test_bbt_start_position() {
        let clock = create_clock(120.0, 44100.0, 4, 4, 4);
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 1, 1));
    }

    #[test]
    fn test_bbt_after_ticks() {
        let mut clock = create_clock(120.0, 44100.0, 4, 4, 4);
        clock.mock_set_tick_counter(5);
        // tick/beat -> 4
        // beat/bar -> 4
        // tick/bar -> 4 * 4 -> 16
        // after 5 tick_counter update, bar -> counter / tick/bar -> 0.3 bar
        // after 5 tick_counter update, beat -> 1 bar = 4 beat -> 4 * bar -> 1.2 beat
        // after 5 tick_counter update, tick -> counter % tick/beat -> 5 % 4 -> 1
        //  but since we use 1-based values, bar -> 1.3(1), beat -> 2.2(2), tick -> 2
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 2, 2));
    }

    #[test]
    fn test_bbt_in_3_4_time() {
        let mut clock = create_clock(120.0, 44100.0, 3, 4, 4);
        clock.mock_set_tick_counter(7);
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 2, 4));
    }

    #[test]
    fn test_bbt_in_6_8_time() {
        let mut clock = create_clock(120.0, 44100.0, 6, 8, 8);
        clock.mock_set_tick_counter(15);
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 2, 8));
    }
}
