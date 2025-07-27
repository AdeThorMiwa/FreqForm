use std::sync::{Arc, Mutex};

use audio_engine::{
    device_manager::{AudioDeviceManager, cpal_dm::CpalAudioDeviceManager},
    mixer::Mixer,
    track::{gainpan::GainPanTrack, sinewave::SineWaveTrack},
};

fn main() {
    // make this into a factory
    let mixer = {
        let mut mixer = Mixer::new();
        let sine = SineWaveTrack::new(440.0, 44100.0);
        let track = GainPanTrack::new(Box::new(sine), 0.3, 0.0);
        mixer.add_track(Box::new(track));
        mixer
    };

    // TODO: Later i’ll upgrade to a lock-free ring buffer.
    let mixer = Arc::new(Mutex::new(mixer));
    let mut manager = CpalAudioDeviceManager::new();

    match manager.start_output_stream(mixer) {
        Ok(_) => {
            println!("Audio stream started.");
            std::thread::park(); // Keep main alive to keep stream alive
        }
        Err(e) => eprintln!("Failed to start audio stream: {:?}", e),
    }
}
