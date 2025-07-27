use std::sync::{Arc, Mutex};

use super::AudioDeviceManager;
use crate::{device_manager::AudioDeviceError, scheduler::Scheduler};
use cpal::{
    Sample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

pub struct CpalAudioDeviceManager {
    stream: Option<cpal::Stream>,
}

impl CpalAudioDeviceManager {
    pub fn new() -> Self {
        Self { stream: None }
    }

    fn build_output_stream<T>(
        &self,
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        scheduler: Arc<Mutex<Scheduler>>,
    ) -> Result<cpal::Stream, AudioDeviceError>
    where
        T: cpal::SizedSample,
        T: cpal::FromSample<f32>,
    {
        let channels = config.channels() as usize;
        let data_cb = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / channels;

            let stereo_samples = {
                let mut sched = scheduler.lock().unwrap();
                sched.next_samples(frames)
            };

            for (i, sample) in data.iter_mut().enumerate() {
                let channel = i % 2; // wrap 
                let raw_sample = if channel == 0 {
                    stereo_samples[i / 2].0
                } else {
                    stereo_samples[i / 2].1
                };
                *sample = raw_sample.to_sample::<T>();
            }
        };

        let error_cb = move |err| {
            eprintln!("Stream error: {}", err);
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
        scheduler: Arc<Mutex<Scheduler>>,
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
                self.build_output_stream::<f32>(&device, config, scheduler)?
            }
            cpal::SampleFormat::I16 => {
                self.build_output_stream::<i16>(&device, config, scheduler)?
            }
            cpal::SampleFormat::U16 => {
                self.build_output_stream::<u16>(&device, config, scheduler)?
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

    #[test]
    fn test_cpal_stream_initializes_successfully() {
        let result = std::panic::catch_unwind(|| {
            let mut manager = CpalAudioDeviceManager::new();
            let scheduler = Arc::new(Mutex::new(Scheduler::new()));
            manager.start_output_stream(scheduler)
        });

        assert!(result.is_ok(), "Stream should start without panicking");
        assert!(result.unwrap().is_ok(), "Stream should start successfully");
    }
}
