use crate::scheduler::{Scheduler, command::SchedulerCommandConsumer};
use std::sync::{Arc, Mutex};

pub mod cpal_dm;

#[derive(Clone, Debug)]
pub enum AudioDeviceError {
    DeviceNotFound,
    StreamBuildFailed(String),
    StreamStartFailed(String),
}

pub trait AudioDeviceManager {
    fn start_output_stream<'a>(
        &mut self,
        mixer: Arc<Mutex<Scheduler>>,
        command_consumer: SchedulerCommandConsumer,
    ) -> Result<(), AudioDeviceError>;
}
