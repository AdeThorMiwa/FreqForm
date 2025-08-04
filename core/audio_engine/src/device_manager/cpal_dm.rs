use super::AudioDeviceManager;
use crate::device_manager::{AudioDeviceError, AudioSource, AudioSourceBufferKind};
use cpal::{
    OutputCallbackInfo,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

pub struct CpalAudioDeviceManager {
    stream: Option<cpal::Stream>,
}

impl CpalAudioDeviceManager {
    pub fn new() -> Self {
        Self { stream: None }
    }

    fn build_output_stream<'a, T, C>(
        &self,
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        mut cb: C,
    ) -> Result<cpal::Stream, AudioDeviceError>
    where
        T: cpal::SizedSample,
        C: FnMut(&mut [T], usize) + Send + 'static,
    {
        let error_cb = move |err| {
            eprintln!("Stream error: {}", err);
        };

        let channels = config.channels() as usize;
        let data_cb = move |data: &mut [T], _: &OutputCallbackInfo| {
            let frame_size = data.len() / channels;
            cb(data, frame_size);
        };

        let stream = device
            .build_output_stream(&config.into(), data_cb, error_cb, None)
            .map_err(|e| AudioDeviceError::StreamBuildFailed(e.to_string()))?;

        Ok(stream)
    }
}

impl AudioDeviceManager for CpalAudioDeviceManager {
    fn start_output_stream(
        &mut self,
        mut audio_source: Box<dyn AudioSource>,
    ) -> Result<(), AudioDeviceError> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or(AudioDeviceError::DeviceNotFound)?;

        let config = device
            .default_output_config()
            .map_err(|e| AudioDeviceError::StreamBuildFailed(e.to_string()))?;

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                self.build_output_stream(&device, config, move |data, frame_size| {
                    audio_source.fill_buffer(AudioSourceBufferKind::F32(data), frame_size)
                })?
            }
            cpal::SampleFormat::I16 => {
                self.build_output_stream(&device, config, move |data, frame_size| {
                    audio_source.fill_buffer(AudioSourceBufferKind::I16(data), frame_size)
                })?
            }
            cpal::SampleFormat::U16 => {
                self.build_output_stream(&device, config, move |data, frame_size| {
                    audio_source.fill_buffer(AudioSourceBufferKind::U16(data), frame_size)
                })?
            }
            format => {
                return Err(AudioDeviceError::StreamBuildFailed(format!(
                    "Unsupported sample format '{format}'"
                )));
            }
        };

        stream
            .play()
            .map_err(|e| AudioDeviceError::StreamStartFailed(e.to_string()))?;

        self.stream = Some(stream);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::Scheduler;
    use rtrb::RingBuffer;

    #[test]
    fn test_cpal_stream_initializes_successfully() {
        let result = std::panic::catch_unwind(|| {
            let mut manager = CpalAudioDeviceManager::new();
            let (_, cons) = RingBuffer::new(1);
            let audio_source = Box::new(Scheduler::new(cons, 44100.0));
            manager.start_output_stream(audio_source)
        });

        assert!(result.is_ok(), "Stream should start without panicking");
        assert!(result.unwrap().is_ok(), "Stream should start successfully");
    }
}
