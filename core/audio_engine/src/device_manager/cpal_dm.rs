use std::sync::{Arc, Mutex};

use super::AudioDeviceManager;
use crate::{
    device_manager::AudioDeviceError,
    scheduler::{Scheduler, command::SchedulerCommandConsumer},
};
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
        mut command_consumer: SchedulerCommandConsumer,
    ) -> Result<cpal::Stream, AudioDeviceError>
    where
        T: cpal::SizedSample,
        T: cpal::FromSample<f32>,
    {
        let channels = config.channels() as usize;
        let data_cb = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let frames = data.len() / channels;

            let mut scheduler = scheduler.lock().unwrap();

            while let Ok(cmd) = command_consumer.pop() {
                scheduler.process_command(cmd);
            }

            let stereo_samples = scheduler.next_samples(frames);

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
    fn start_output_stream<'a>(
        &mut self,
        scheduler: Arc<Mutex<Scheduler>>,
        command_consumer: SchedulerCommandConsumer,
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
                self.build_output_stream::<f32>(&device, config, scheduler, command_consumer)?
            }
            cpal::SampleFormat::I16 => {
                self.build_output_stream::<i16>(&device, config, scheduler, command_consumer)?
            }
            cpal::SampleFormat::U16 => {
                self.build_output_stream::<u16>(&device, config, scheduler, command_consumer)?
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
            let (_, cons) = rtrb::RingBuffer::new(10);
            manager.start_output_stream(scheduler, cons)
        });

        assert!(result.is_ok(), "Stream should start without panicking");
        assert!(result.unwrap().is_ok(), "Stream should start successfully");
    }
}
