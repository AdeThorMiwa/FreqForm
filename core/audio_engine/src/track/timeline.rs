use crate::{
    clip::{Clip, ClipKind, clip_id::ClipId},
    track::TrackId,
};
use std::sync::Arc;

/// TimelineTrack holds a list of clips and plays them according to the timeline
#[derive(Debug)]
pub struct TimelineTrack {
    pub id: TrackId,
    pub name: String,
    pub clips: Vec<Clip>,
}

impl TimelineTrack {
    pub fn new(id: TrackId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            clips: Vec::new(),
        }
    }

    pub fn add_clip(&mut self, clip: Clip) {
        self.clips.push(clip);
        self.clips.sort_by_key(|c| c.timing.start_frame);
    }

    pub fn remove_clip(&mut self, clip_id: ClipId) {
        self.clips.retain(|c| c.id != clip_id);
    }

    /// Render N frames of audio starting from `start_frame` into `output_buffer`
    pub fn render_audio(
        &mut self,
        start_frame: u64,
        frame_count: usize,
        output_buffer: &mut [(f32, f32)],
    ) {
        for clip in self.clips.iter_mut() {
            if !clip.is_active_at(start_frame) {
                continue;
            }

            match &mut clip.content {
                ClipKind::Audio(audio_clip) => {
                    let source = Arc::clone(&audio_clip.source);
                    let start_offset = audio_clip.start_offset;
                    let gain = audio_clip.gain;
                    let pan = audio_clip.pan;

                    // Calculate read position from source (considering start_frame and clip offset)
                    let rel_frame = start_frame.saturating_sub(clip.timing.start_frame);
                    let read_start = start_offset + rel_frame;

                    let samples = source.read_samples(read_start, frame_count);

                    for (i, (l, r)) in samples.iter().enumerate().take(frame_count) {
                        let (l, r) = apply_gain_pan(*l, *r, gain, pan);
                        output_buffer[i].0 += l;
                        output_buffer[i].1 += r;
                    }
                } //_ => {}
            }
        }
    }
}

fn apply_gain_pan(l: f32, r: f32, gain: f32, pan: f32) -> (f32, f32) {
    let gain = gain.clamp(0.0, 4.0);
    let pan = pan.clamp(-1.0, 1.0);

    let pan_l = if pan < 0.0 { 1.0 } else { 1.0 - pan };
    let pan_r = if pan > 0.0 { 1.0 } else { 1.0 + pan };

    (l * gain * pan_l, r * gain * pan_r)
}
