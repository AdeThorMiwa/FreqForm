use audio_engine::device_manager::{AudioDeviceManager, cpal_dm::CpalAudioDeviceManager};

fn main() {
    // make this into a factory
    let mut manager = CpalAudioDeviceManager::new();

    match manager.start_output_stream() {
        Ok(_) => {
            println!("🎧 Audio stream started. Playing silence...");
            std::thread::park(); // Keep main alive to keep stream alive
        }
        Err(e) => eprintln!("❌ Failed to start audio stream: {:?}", e),
    }
}
