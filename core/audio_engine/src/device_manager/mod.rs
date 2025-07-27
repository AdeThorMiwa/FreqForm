use std::sync::{Arc, Mutex};

use crate::mixer::Mixer;

pub mod cpal_dm;

#[derive(Clone, Debug)]
pub enum AudioDeviceError {
    DeviceNotFound,
    StreamBuildFailed(String),
    StreamStartFailed(String),
}

pub trait AudioDeviceManager {
    fn start_output_stream(&mut self, mixer: Arc<Mutex<Mixer>>) -> Result<(), AudioDeviceError>;
}
