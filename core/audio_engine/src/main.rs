use std::sync::{Arc, Mutex};

use audio_engine::{
    device_manager::{AudioDeviceManager, cpal_dm::CpalAudioDeviceManager},
    mixer::Mixer,
    track::{gainpan::GainPanTrack, wav::WavTrack},
};

fn main() {
    let mixer = {
        let mut mixer = Mixer::new();
        let wav = WavTrack::from_file("./assets/wav/piano.wav").expect("Failed to load WAV");
        let track = GainPanTrack::new(Box::new(wav), 0.8, -0.3);
        mixer.add_track(Box::new(track));
        mixer
    };

    // TODO: Later i’ll upgrade to a lock-free ring buffer.
    let mixer = Arc::new(Mutex::new(mixer));
    // make this into a factory
    let mut manager = CpalAudioDeviceManager::new();

    match manager.start_output_stream(mixer) {
        Ok(_) => {
            println!("Audio stream started.");
            std::thread::park(); // Keep main alive to keep stream alive
        }
        Err(e) => eprintln!("Failed to start audio stream: {:?}", e),
    }
}
