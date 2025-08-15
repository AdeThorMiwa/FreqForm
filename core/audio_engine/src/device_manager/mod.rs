pub mod cpal_dm;

#[derive(Clone, Debug)]
pub enum AudioDeviceError {
    DeviceNotFound,
    StreamBuildFailed(String),
    StreamStartFailed(String),
}

pub enum AudioSourceBufferKind<'a> {
    F32(&'a mut [f32]),
    I16(&'a mut [i16]),
    U16(&'a mut [u16]),
}

pub trait AudioSource
where
    Self: Send,
    Self: std::fmt::Debug,
{
    fn fill_buffer(&mut self, buffer: AudioSourceBufferKind<'_>, frame_size: usize);
}

pub trait AudioDeviceManager {
    fn start_output_stream(
        &mut self,
        audio_source: Box<dyn AudioSource>,
    ) -> Result<(), AudioDeviceError>;
}
