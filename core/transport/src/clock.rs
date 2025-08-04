use crate::resolution::TickResolution;

#[derive(Debug, Clone, Copy)]
pub struct TimeSignature {
    pub beats_per_bar: u64, // numerator (e.g., 4 in 4/4)
    pub beat_unit: u64,     // denominator (e.g., 4 in 4/4)
}

// @todo move to core::transport
pub struct TempoClock {
    bpm: f64,
    samples_per_tick: f64,
    sample_position: f64,
    tick_counter: u64,
    running: bool,
    pub time_signature: TimeSignature,
    pub ticks_per_beat: u64,
    sample_rate: f64,
}

impl TempoClock {
    pub fn new(bpm: f64, sample_rate: f64, resolution: TickResolution) -> Self {
        let time_signature = TimeSignature {
            beats_per_bar: 4,
            beat_unit: 4,
        };
        Self::with_signature(bpm, sample_rate, resolution, time_signature)
    }

    fn compute_samples_per_tick(bpm: f64, sample_rate: f64, ticks_per_beat: u64) -> f64 {
        let seconds_per_beat = 60.0 / bpm;
        let seconds_per_tick = seconds_per_beat / ticks_per_beat as f64;
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

        println!(
            "tick counter: {} spt: {} bpm: {}",
            self.tick_counter, self.samples_per_tick, self.bpm
        );

        tick_emitted
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_counter
    }

    pub fn tick_phase(&self) -> f64 {
        self.sample_position / self.samples_per_tick
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
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
    ) -> Self {
        let ticks_per_beat = resolution.ticks_per_beat();
        let samples_per_tick =
            TempoClock::compute_samples_per_tick(bpm, sample_rate, ticks_per_beat);
        Self {
            bpm,
            samples_per_tick,
            sample_position: 0.0,
            tick_counter: 0,
            running: true,
            time_signature,
            ticks_per_beat,
            sample_rate,
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
        // sample_rate / (bpm / 60 * ticks_per_beat)
        // samples_per_tick = 44100 / (120.0 / 60 * 120) = 183.75
        assert!((clock.samples_per_tick() - 183.75).abs() < 0.01);
    }

    #[test]
    fn test_no_tick_emitted_before_threshold() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        let tick_emitted = clock.advance_by(40); // threshold is 45
        assert!(!tick_emitted);
    }

    #[test]
    fn test_tick_emitted_at_threshold() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        // 120 BPM -> 0.5s per quarter -> 22050 samples
        let tick_emitted = clock.advance_by(22050);
        assert!(tick_emitted);
        assert_eq!(clock.current_tick(), 480);
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
        assert_eq!(clock.current_tick(), 960);
        assert_eq!(ticks, 2);
    }

    #[test]
    fn test_tick_phase_accuracy() {
        let mut clock = TempoClock::new(120.0, SAMPLE_RATE, TickResolution::Quarter);
        clock.advance_by(11025); // half a quarter note
        let phase = clock.tick_phase();
        assert!((phase - 0.0).abs() < 0.01);
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
        resolution: TickResolution,
    ) -> TempoClock {
        TempoClock::with_signature(
            bpm,
            sample_rate,
            resolution,
            TimeSignature {
                beats_per_bar,
                beat_unit,
            },
        )
    }

    #[test]
    fn test_bbt_start_position() {
        let clock = create_clock(120.0, 44100.0, 4, 4, TickResolution::Sixteenth);
        let (bar, beat, tick) = clock.bar_beat_tick();
        // tick/beat -> 120
        // beat/bar -> 4
        // tick/bar -> 4 * 120 -> 480
        // after 5 tick_counter update, bar -> counter / tick/bar -> 0.0 bar
        // after 5 tick_counter update, beat -> 1 bar = 4 beat -> 4 * bar -> 0 beat
        // after 5 tick_counter update, tick -> counter % tick/beat -> 0 % 120 -> 0
        //  but since we use 1-based values, bar -> 1, beat -> 1, tick -> 1
        assert_eq!((bar, beat, tick), (1, 1, 1));
    }

    #[test]
    fn test_bbt_after_ticks() {
        let mut clock = create_clock(120.0, 44100.0, 4, 4, TickResolution::Sixteenth);
        clock.mock_set_tick_counter(5);
        // tick/beat -> 120
        // beat/bar -> 4
        // tick/bar -> 4 * 120 -> 480
        // after 5 tick_counter update, bar -> counter / tick/bar -> 0.0 bar
        // after 5 tick_counter update, beat -> 1 bar = 4 beat -> 4 * bar -> 0 beat
        // after 5 tick_counter update, tick -> counter % tick/beat -> 5 % 120 -> 5
        //  but since we use 1-based values, bar -> 1, beat -> 1, tick -> 6
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 1, 6));
    }

    #[test]
    fn test_bbt_in_3_4_time() {
        let mut clock = create_clock(120.0, 44100.0, 3, 4, TickResolution::Sixteenth);
        clock.mock_set_tick_counter(7);
        // tick/beat -> 120
        // beat/bar -> 3
        // tick/bar -> 3 * 120 -> 360
        // after 5 tick_counter update, bar -> counter / tick/bar -> 0.0 bar
        // after 5 tick_counter update, beat -> 1 bar = 3 beat -> 3 * bar -> 0 beat
        // after 5 tick_counter update, tick -> counter % tick/beat -> 7 % 120 -> 7
        //  but since we use 1-based values, bar -> 1, beat -> 1, tick -> 8
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 1, 8));
    }

    #[test]
    fn test_bbt_in_6_8_time() {
        let mut clock = create_clock(120.0, 44100.0, 6, 8, TickResolution::Eighth);
        clock.mock_set_tick_counter(15);
        let (bar, beat, tick) = clock.bar_beat_tick();
        assert_eq!((bar, beat, tick), (1, 1, 16));
    }
}
