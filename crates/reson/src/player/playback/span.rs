use crate::timebase::{frames_to_seconds, seconds_to_frames_round};

pub(super) struct QuantizedSpan {
    pub(super) track_frames: u64,
    pub(super) start_frame: u64,
    pub(super) end_frame: u64,
    pub(super) frames: u64,
    pub(super) samples: u64,
    pub(super) start_seconds: f32,
    pub(super) end_seconds: f32,
    channels: u16,
}

impl QuantizedSpan {
    pub(super) fn new(
        start_seconds: f32,
        end_seconds: f32,
        track_duration: f32,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        let (track_frames, start_frame, end_frame) =
            quantize_span_bounds(start_seconds, end_seconds, track_duration, sample_rate);
        let frames = end_frame.saturating_sub(start_frame).max(1);
        let samples = frames.saturating_mul(channels as u64);

        Self {
            track_frames,
            start_frame,
            end_frame,
            frames,
            samples,
            start_seconds: frames_to_seconds(start_frame, sample_rate),
            end_seconds: frames_to_seconds(end_frame, sample_rate),
            channels,
        }
    }

    pub(super) fn start_sample(&self) -> usize {
        self.start_frame.saturating_mul(self.channels as u64) as usize
    }
}

fn quantize_span_bounds(
    start_seconds: f32,
    end_seconds: f32,
    track_duration: f32,
    sample_rate: u32,
) -> (u64, u64, u64) {
    let track_frames = seconds_to_frames_round(track_duration.max(0.0), sample_rate).max(1);
    let mut start_frame =
        seconds_to_frames_round(start_seconds.max(0.0), sample_rate).min(track_frames - 1);
    let mut end_frame =
        seconds_to_frames_round(end_seconds.max(0.0), sample_rate).min(track_frames);

    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(track_frames);
    }
    if end_frame <= start_frame {
        start_frame = 0;
        end_frame = 1.min(track_frames);
    }
    (track_frames, start_frame, end_frame)
}
