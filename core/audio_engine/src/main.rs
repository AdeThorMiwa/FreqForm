use audio_engine::{
    device_manager::{AudioDeviceManager, cpal_dm::CpalAudioDeviceManager},
    scheduler::{
        Scheduler,
        command::{ParameterChange, SchedulerCommand},
    },
    track::{gainpan::GainPanTrack, wav::WavTrack},
};

fn main() {
    let (mut prod, cons) = rtrb::RingBuffer::<SchedulerCommand>::new(128);
    let audio_source = Box::new(Scheduler::new(cons));
    // make this into a factory
    let mut manager = CpalAudioDeviceManager::new();

    manager
        .start_output_stream(audio_source)
        .expect("Failed to start audio stream");

    println!("Stream started");

    let piano = {
        let wav = WavTrack::from_file("./assets/wav/piano.wav").expect("Failed to load WAV");
        GainPanTrack::new("x-track", Box::new(wav), 0.1, 1.0)
    };

    let time_to_frame = |time_in_sec: f64| {
        let sample_rate = 44100.0;
        (time_in_sec * sample_rate) as u64
    };

    prod.push(SchedulerCommand::ScheduleTrack {
        track: Box::new(piano),
        start_frame: time_to_frame(1.0),
    })
    .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("Lowering gain to 0.3");

    prod.push(SchedulerCommand::ParamChange {
        target_id: "x-track".into(),
        change: ParameterChange::SetGain(1.0),
    })
    .unwrap();

    // Change pan to -1.0 (left) after another 2s
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("Panning fully left");
    prod.push(SchedulerCommand::ParamChange {
        target_id: "x-track".into(),
        change: ParameterChange::SetPan(-1.0),
    })
    .unwrap();

    std::thread::park();
}
