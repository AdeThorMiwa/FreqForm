use crate::resolution::QuantizeResolution;

pub struct Quantizer;

impl Quantizer {
    /// Snap to nearest tick on the quantization grid
    pub fn quantize_tick(tick: u64, resolution: QuantizeResolution, ticks_per_beat: u64) -> u64 {
        let grid_size = resolution.ticks_per_grid_unit(ticks_per_beat);
        ((tick as f64 / grid_size as f64).round() as u64) * grid_size
    }

    /// Always quantize forward to next grid position
    pub fn quantize_tick_forward(
        tick: u64,
        resolution: QuantizeResolution,
        ticks_per_beat: u64,
    ) -> u64 {
        let grid_size = resolution.ticks_per_grid_unit(ticks_per_beat);
        ((tick + grid_size - 1) / grid_size) * grid_size
    }
}

#[cfg(test)]
mod quantizer_tests {
    use super::*;

    #[test]
    fn test_snap_to_nearest_16th_note() {
        let ticks_per_beat = 4;
        let tick = 7;
        let quantized =
            Quantizer::quantize_tick(tick, QuantizeResolution::Sixteenth, ticks_per_beat);
        assert_eq!(quantized, 7);
    }

    #[test]
    fn test_snap_to_nearest_8th_note() {
        let ticks_per_beat = 4;
        let tick = 6;
        let quantized = Quantizer::quantize_tick(tick, QuantizeResolution::Eighth, ticks_per_beat);
        assert_eq!(quantized, 6);
    }

    #[test]
    fn test_forward_quantize_16th_note() {
        let ticks_per_beat = 4;
        let tick = 7;
        let quantized =
            Quantizer::quantize_tick_forward(tick, QuantizeResolution::Sixteenth, ticks_per_beat);
        assert_eq!(quantized, 7);
    }

    #[test]
    fn test_forward_quantize_8th_note_exact() {
        let ticks_per_beat = 4;
        let tick = 8;
        let quantized =
            Quantizer::quantize_tick_forward(tick, QuantizeResolution::Eighth, ticks_per_beat);
        assert_eq!(quantized, 8);
    }

    #[test]
    fn test_high_resolution_quantization() {
        let ticks_per_beat = 960;
        let tick = 144; // halfway between 120 and 240
        let snap = Quantizer::quantize_tick(tick, QuantizeResolution::Sixteenth, ticks_per_beat);
        let forward =
            Quantizer::quantize_tick_forward(tick, QuantizeResolution::Sixteenth, ticks_per_beat);

        assert_eq!(snap, 240); // Nearest 16th note (960 / 4 = 240 per 16th, nearest is 120)
        assert_eq!(forward, 240); // Always forward to 240
    }
}
