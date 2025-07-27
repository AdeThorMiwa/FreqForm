use std::sync::{Arc, Mutex};

use audio_engine::{
    device_manager::{AudioDeviceManager, cpal_dm::CpalAudioDeviceManager},
    scheduler::Scheduler,
    track::{gainpan::GainPanTrack, wav::WavTrack},
};

fn main() {
    // let mixer = {
    //     let mut mixer = Mixer::new();
    //     let wav = WavTrack::from_file("./assets/wav/piano.wav").expect("Failed to load WAV");
    //     let track = GainPanTrack::new(Box::new(wav), 0.8, -0.3);
    //     mixer.add_track(Box::new(track));
    //     mixer
    // };

    let scheduler = {
        let mut sched = Scheduler::new();
        let piano = {
            let wav = WavTrack::from_file("./assets/wav/piano.wav").expect("Failed to load WAV");
            GainPanTrack::new(Box::new(wav), 0.4, 0.0)
        };
        let dark = {
            let wav = WavTrack::from_file("./assets/wav/dark.wav").expect("Failed to load WAV");
            GainPanTrack::new(Box::new(wav), 2.0, 0.0)
        };

        let time_to_frame = |time_in_sec: f64| {
            let sample_rate = 44100.0;
            (time_in_sec * sample_rate) as u64
        };

        sched.schedule(Box::new(piano), time_to_frame(3.0));
        sched.schedule(Box::new(dark), time_to_frame(0.0));

        sched
    };

    // TODO: Later i’ll upgrade to a lock-free ring buffer.
    // TODO: abstract this into an interface that expose a readable stream for
    // output manager to consume
    let stream = Arc::new(Mutex::new(scheduler));
    // make this into a factory
    let mut manager = CpalAudioDeviceManager::new();

    match manager.start_output_stream(stream) {
        Ok(_) => {
            println!("Audio stream started.");
            std::thread::park(); // Keep main alive to keep stream alive
        }
        Err(e) => eprintln!("Failed to start audio stream: {:?}", e),
    }
}
