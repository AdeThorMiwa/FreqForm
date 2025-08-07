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
                    let offset = audio_clip.start_offset;
                    let gain = audio_clip.gain;
                    let pan = audio_clip.pan;
                    let should_loop = audio_clip.looping;

                    let clip_start = clip.timing.start_frame;
                    let clip_end = clip.ends_at();
                    let clip_length = clip.timing.duration_frames;

                    for i in 0..frame_count {
                        let global_frame = start_frame + i as u64;

                        if global_frame < clip_start {
                            continue;
                        }

                        if !should_loop && global_frame >= clip_end {
                            continue;
                        }

                        let clip_relative = global_frame - clip_start;

                        // Loop offset within clip
                        let local_frame = if should_loop {
                            clip_relative % clip_length
                        } else {
                            clip_relative
                        };

                        let source_frame = offset + local_frame;

                        let sample = source
                            .read_samples(source_frame, 1)
                            .get(0)
                            .copied()
                            .unwrap_or((0.0, 0.0));
                        let (l, r) = apply_gain_pan(sample.0, sample.1, gain, pan);

                        output_buffer[i].0 += l;
                        output_buffer[i].1 += r;
                    }
                }
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
