pub mod cpal_dm;

#[derive(Clone, Debug)]
pub enum AudioDeviceError {
    DeviceNotFound,
    StreamBuildFailed(String),
    StreamStartFailed(String),
}

pub trait AudioDeviceManager {
    fn start_output_stream(&mut self) -> Result<(), AudioDeviceError>;
}
