use std::sync::{Arc, Mutex};

use audio_engine::{
    device_manager::{AudioDeviceManager, cpal_dm::CpalAudioDeviceManager},
    scheduler::{Scheduler, command::SchedulerCommand},
    track::{gainpan::GainPanTrack, wav::WavTrack},
};

fn main() {
    let (mut prod, cons) = rtrb::RingBuffer::<SchedulerCommand>::new(128);
    let scheduler = Scheduler::new();

    // TODO: abstract this into an interface that expose a readable stream for
    // output manager to consume
    let stream = Arc::new(Mutex::new(scheduler));
    // make this into a factory
    let mut manager = CpalAudioDeviceManager::new();

    manager
        .start_output_stream(stream.clone(), cons)
        .expect("Failed to start audio stream");

    println!("Stream started");

    std::thread::sleep(std::time::Duration::from_secs(2));

    let piano = {
        let wav = WavTrack::from_file("./assets/wav/piano.wav").expect("Failed to load WAV");
        GainPanTrack::new(Box::new(wav), 0.4, 0.0)
    };

    let time_to_frame = |time_in_sec: f64| {
        let sample_rate = 44100.0;
        (time_in_sec * sample_rate) as u64
    };

    prod.push(SchedulerCommand::ScheduleTrack {
        track: Box::new(piano),
        start_frame: time_to_frame(1.0),
    })
    .expect("Push failed: Buffer full");

    std::thread::park();
}
