use crate::{
    clip::{
        Clip, ClipKind,
        clip_id::ClipId,
        fades::{Fade, FadeCurve},
    },
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
            match &mut clip.kind {
                ClipKind::Audio(audio_clip) => {
                    let source = Arc::clone(&audio_clip.source);
                    let offset = audio_clip.start_offset;
                    let gain = audio_clip.gain;
                    let pan = audio_clip.pan;
                    let should_loop = audio_clip.looping;

                    let clip_start = clip.timing.start_frame;
                    let clip_end = clip.ends_at();
                    let clip_length = clip.timing.duration_frames;
                    let fi = clip.fade_in;
                    let fo = clip.fade_out;

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
                        let relative_time_frame = clip_start + local_frame;

                        if !clip.is_active_at(relative_time_frame) {
                            continue;
                        }

                        // Read one sample (assume read_samples is cheap or backed by cache/streamer)
                        let (mut l, mut r) = source
                            .read_samples(source_frame, 1)
                            .get(0)
                            .copied()
                            .unwrap_or((0.0, 0.0));

                        // Fade gain
                        let fg = compute_fade_gain(local_frame, clip_length, fi, fo);

                        // Apply fade, then gain/pan
                        l *= fg;
                        r *= fg;

                        let (l, r) = apply_gain_pan(l, r, gain, pan);

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

#[inline]
fn fade_gain_equal_power_in(i: u64, n: u64) -> f32 {
    if n == 0 {
        return 1.0;
    }
    let t = (i as f32) / (n as f32);
    (std::f32::consts::FRAC_PI_2 * t).sin()
}

#[inline]
fn fade_gain_equal_power_out(i_from_end: u64, n: u64) -> f32 {
    if n == 0 {
        return 1.0;
    }
    let p = 1.0 - (i_from_end as f32) / (n as f32);
    (std::f32::consts::FRAC_PI_2 * p.clamp(0.0, 1.0)).cos()
}

#[inline]
fn fade_gain_linear_in(i: u64, n: u64) -> f32 {
    if n == 0 {
        return 1.0;
    }
    (i as f32) / (n as f32)
}

#[inline]
fn fade_gain_linear_out(i_from_end: u64, n: u64) -> f32 {
    if n == 0 {
        return 1.0;
    }
    (i_from_end as f32) / (n as f32)
}

#[inline]
fn compute_fade_gain(local_frame: u64, clip_len: u64, fade_in: Fade, fade_out: Fade) -> f32 {
    let mut g = 1.0f32;

    // fade-in (apply if inside fade-in zone)
    if fade_in.length_frames > 0 && local_frame < fade_in.length_frames {
        g = match fade_in.curve {
            FadeCurve::Linear => fade_gain_linear_in(local_frame, fade_in.length_frames),
            FadeCurve::EqualPower => fade_gain_equal_power_in(local_frame, fade_in.length_frames),
        };
    }

    // fade-out (apply if inside fade-out zone)
    if fade_out.length_frames > 0 {
        let out_start = clip_len.saturating_sub(fade_out.length_frames);
        if local_frame >= out_start {
            let from_end = (clip_len - 1).saturating_sub(local_frame);
            let go = match fade_out.curve {
                FadeCurve::Linear => fade_gain_linear_out(from_end, fade_out.length_frames),
                FadeCurve::EqualPower => {
                    fade_gain_equal_power_out(from_end, fade_out.length_frames)
                }
            };
            // If both in & out apply (tiny clips), use the *minimum* to avoid >1.0 boosts
            g = g.min(go);
        }
    }
    g
}
