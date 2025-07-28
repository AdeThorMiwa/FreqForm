use std::{io::Read, path::Path};

use hound::WavReader;

use crate::track::Track;

/// Loads a .wav file and exposes a Track interface to consume samples.
///
/// NOTE: Only support `16 bit per sample` for wav files with `Int` sample format for now
pub struct WavTrack {
    pub(crate) samples: Vec<(f32, f32)>,
    pub(crate) position: usize,
}

impl WavTrack {
    fn new<R: Read + Send + 'static>(reader: WavReader<R>) -> Result<Self, String> {
        let spec = reader.spec();
        let channels = spec.channels;
        if channels == 0 || channels > 2 {
            return Err("Only mono or stereo WAVs are supported".into());
        }

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .into_samples::<i16>()
                .filter_map(Result::ok)
                .map(|s| s as f32 / i16::MAX as f32)
                .collect(),
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(Result::ok)
                .collect(),
        };

        let stereo_samples: Vec<(f32, f32)> = if channels == 1 {
            samples.iter().map(|&s| (s, s)).collect()
        } else {
            samples
                .chunks(2)
                .map(|chunk| (chunk[0], chunk[1]))
                .collect()
        };

        Ok(Self {
            samples: stereo_samples,
            position: 0,
        })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let reader =
            WavReader::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
        Self::new(reader)
    }

    pub fn from_reader<R: Read + Send + 'static>(reader: R) -> Result<Self, String> {
        let reader =
            WavReader::new(reader).map_err(|e| format!("Failed to open WAV file: {}", e))?;
        Self::new(reader)
    }
}

impl Track for WavTrack {
    fn id(&self) -> String {
        "wav-track".to_owned()
    }

    fn next_samples(&mut self, frame_size: usize) -> Vec<(f32, f32)> {
        let end = (self.position + frame_size).min(self.samples.len());
        let slice = &self.samples[self.position..end];
        self.position = end;

        // If we’re at EOF, return silence
        let mut result: Vec<(f32, f32)> = slice.to_vec();
        while result.len() < frame_size {
            result.push((0.0, 0.0));
        }

        result
    }

    fn reset(&mut self) {
        self.position = 0;
    }
}

#[cfg(test)]
mod tests {
    use crate::constants::AUDIO_SAMPLE_EPSILON;

    use super::*;
    use hound::WavSpec;
    use std::io::Cursor;

    fn create_wav_buffer(spec: WavSpec, samples: &[i16]) -> Cursor<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut buffer, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
        buffer.set_position(0);
        buffer
    }

    #[test]
    fn test_mono_wav_expands_to_stereo() {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let samples = [1000, -1000]; // short mono buffer
        let buffer = create_wav_buffer(spec, &samples);
        let mut track = WavTrack::from_reader(buffer).unwrap();

        let output = track.next_samples(2);
        assert_eq!(output.len(), 2);
        assert!((output[0].0 - output[0].1).abs() < AUDIO_SAMPLE_EPSILON); // L = R
        assert!((output[1].0 - output[1].1).abs() < AUDIO_SAMPLE_EPSILON);
    }

    #[test]
    fn test_returns_silence_after_end_of_file() {
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let samples = [2000];
        let buffer = create_wav_buffer(spec, &samples);
        let mut track = WavTrack::from_reader(buffer).unwrap();

        let output = track.next_samples(3); // request more than exists
        assert_eq!(output.len(), 3);
        assert_ne!(output[0], (0.0, 0.0)); // actual sample
        assert_eq!(output[1], (0.0, 0.0)); // padded silence
        assert_eq!(output[2], (0.0, 0.0));
    }

    #[test]
    fn test_invalid_channels_should_fail() {
        let spec = WavSpec {
            channels: 3, // unsupported
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let samples = [0; 6];
        let buffer = create_wav_buffer(spec, &samples);
        let result = WavTrack::from_reader(buffer);
        assert!(result.is_err());
    }
}
