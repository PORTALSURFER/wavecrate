use super::*;

impl WaveformRenderer {
    /// Sample a value at an interpolated frame position.
    ///
    /// The selected position is clamped to available samples. Channel selection uses
    /// clamped indexing so malformed input channel requests remain safe.
    pub(super) fn sample_at_frame(
        samples: &[f32],
        channels: usize,
        frame_pos: f32,
        channel_index: Option<usize>,
    ) -> f32 {
        let frame_count = samples.len() / channels.max(1);
        if frame_count == 0 {
            return 0.0;
        }
        let frame_pos = frame_pos.clamp(0.0, (frame_count - 1) as f32);
        let i0 = frame_pos.floor() as usize;
        let i1 = (i0 + 1).min(frame_count - 1);
        let t = frame_pos - i0 as f32;
        let sample_at_channel = |frame: usize, channel: usize| -> f32 {
            let base = frame * channels;
            samples
                .get(base + channel.min(channels.saturating_sub(1)))
                .copied()
                .unwrap_or(0.0)
        };
        let interpolated_for_channel = |channel: usize| -> f32 {
            if i0 >= 1 && i1 + 1 < frame_count {
                let p0 = sample_at_channel(i0 - 1, channel);
                let p1 = sample_at_channel(i0, channel);
                let p2 = sample_at_channel(i1, channel);
                let p3 = sample_at_channel(i1 + 1, channel);
                return Self::catmull_rom(p0, p1, p2, p3, t);
            }
            let a = sample_at_channel(i0, channel);
            let b = sample_at_channel(i1, channel);
            a + (b - a) * t
        };
        match channel_index {
            Some(channel) => interpolated_for_channel(channel),
            None => {
                let mut chosen = 0.0_f32;
                let mut best = -1.0_f32;
                for channel in 0..channels.max(1) {
                    let sample = interpolated_for_channel(channel);
                    let score = sample.abs();
                    if score > best {
                        best = score;
                        chosen = sample;
                    }
                }
                chosen
            }
        }
    }

    /// Return a supersampled sample for a single output column.
    ///
    /// Uses a fixed 8-sample subdivision within each column and interpolates each
    /// sample point before averaging to reduce aliasing.
    pub(super) fn supersampled_frame(
        samples: &[f32],
        channels: usize,
        frame_count: usize,
        x: usize,
        width: usize,
        channel_index: Option<usize>,
    ) -> f32 {
        if width <= 1 || frame_count == 0 {
            return Self::sample_at_frame(samples, channels, 0.0, channel_index);
        }
        let sub_samples = 8;
        let mut sum = 0.0_f32;
        for i in 0..sub_samples {
            let offset = (i as f32 + 0.5) / sub_samples as f32;
            let t = (x as f32 + offset) / (width as f32 - 1.0);
            let frame_pos = t * (frame_count.saturating_sub(1)) as f32;
            sum += Self::sample_at_frame(samples, channels, frame_pos, channel_index);
        }
        sum / sub_samples as f32
    }

    /// Evaluate a Catmull-Rom cubic segment for interpolation.
    pub(super) fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * (2.0 * p1
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }
}
