use std::{io::Read, path::Path};

use hound::WavReader;
use uuid::Uuid;

use crate::track::{Track, TrackId};

/// `WavTrack` represents an in-memory, stereo-normalized PCM buffer loaded from a `.wav` file.
///
/// Supports:
/// - Mono and Stereo files (mono is duplicated into both channels)
/// - 16-bit integer or 32-bit float samples (converted to `f32`)
///
/// Does NOT support:
/// - More than 2 channels
/// - Sample rates ≠ project sample rate (no resampling yet)
///
/// # Example
/// ```no_run
/// use audio_engine::track::wav::WavTrack;
///
/// let track = WavTrack::from_file("assets/wav/piano.wav").unwrap();
/// ```
#[derive(Debug)]
pub struct WavTrack {
    /// track id
    id: TrackId,
    /// file name
    name: String,
    /// Interleaved stereo frames
    samples: Vec<(f32, f32)>,
    /// Current read position (frame index)
    position: usize,
}

impl WavTrack {
    fn from_reader<R: Read + Send + 'static>(
        reader: WavReader<R>,
        name: &str,
    ) -> Result<Self, String> {
        let spec = reader.spec();
        let channels = spec.channels;
        if channels == 0 || channels > 2 {
            return Err("Only mono or stereo WAVs are supported".into());
        }

        let pcm_samples = Self::decode_pcm_samples(reader)?;
        Ok(Self {
            id: Uuid::new_v4().into(),
            name: name.to_owned(),
            samples: pcm_samples,
            position: 0,
        })
    }

    pub fn from_file<P: AsRef<Path> + Clone>(path: P) -> Result<Self, String> {
        let name = {
            let p = path.clone();
            p.as_ref()
                .file_name()
                .clone()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
        };
        let reader =
            WavReader::open(path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
        Self::from_reader(reader, &name)
    }

    pub fn from_stream<R: Read + Send + 'static>(stream: R) -> Result<Self, String> {
        let reader =
            WavReader::new(stream).map_err(|e| format!("Failed to parse WAV stream: {}", e))?;
        Self::from_reader(reader, "stream")
    }

    fn decode_pcm_samples<R: Read + Send + 'static>(
        reader: WavReader<R>,
    ) -> Result<Vec<(f32, f32)>, String> {
        let spec = reader.spec();
        let raw_samples = match spec.sample_format {
            hound::SampleFormat::Int => reader
                .into_samples::<i16>()
                .filter_map(Result::ok)
                .map(|s| s as f32 / i16::MAX as f32)
                .collect::<Vec<f32>>(),
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(Result::ok)
                .collect::<Vec<f32>>(),
        };

        Ok(Self::interleave_channels(
            raw_samples,
            spec.channels as usize,
        ))
    }

    /// Converts raw f32 samples into stereo `(L, R)` frames.
    /// Mono is duplicated into both channels.
    fn interleave_channels(samples: Vec<f32>, channels: usize) -> Vec<(f32, f32)> {
        match channels {
            1 => samples.into_iter().map(|s| (s, s)).collect(),
            2 => samples
                .chunks_exact(2)
                .map(|chunk| (chunk[0], chunk[1]))
                .collect(),
            _ => unreachable!("Unsupported channel count"),
        }
    }

    #[cfg(test)]
    pub fn from_raw_samples(samples: Vec<(f32, f32)>) -> Self {
        Self {
            id: Uuid::new_v4().into(),
            name: "raw-samples.wav".to_owned(),
            position: 0,
            samples,
        }
    }
}

impl Track for WavTrack {
    fn id(&self) -> TrackId {
        self.id.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn track_type(&self) -> super::TrackType {
        super::TrackType::Audio
    }

    fn fill_next_samples(&mut self, next_samples: &mut [(f32, f32)]) {
        let end = (self.position + next_samples.len()).min(self.samples.len());
        let _ = &next_samples[..(end - self.position)]
            .copy_from_slice(&self.samples[self.position..end]);
        self.position = end;
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
        let mut track = WavTrack::from_stream(buffer).unwrap();

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
        let mut track = WavTrack::from_stream(buffer).unwrap();

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
        let result = WavTrack::from_stream(buffer);
        assert!(result.is_err());
    }
}
