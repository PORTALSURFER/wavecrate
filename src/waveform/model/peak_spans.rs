//! Pure peak-span and viewport sampling helpers for waveform model types.

use super::{DecodedWaveform, WaveformChannelView, WaveformColumnView, WaveformPeaks};
impl DecodedWaveform {
    pub(crate) fn max_abs_in_span(&self, start: f32, end: f32) -> Option<f32> {
        if !start.is_finite() || !end.is_finite() {
            return None;
        }
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let total_frames = self.frame_count();
        if total_frames == 0 {
            return None;
        }
        if let Some(peaks) = self.peaks.as_deref() {
            return max_abs_from_peaks(peaks, start, end);
        }
        max_abs_from_samples(&self.samples, self.channel_count(), start, end)
    }
}

fn max_abs_from_samples(samples: &[f32], channels: usize, start: f32, end: f32) -> Option<f32> {
    if samples.is_empty() {
        return None;
    }
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return None;
    }
    let start_frame = (start.clamp(0.0, 1.0) * total_frames as f32).floor() as usize;
    let mut end_frame = (end.clamp(0.0, 1.0) * total_frames as f32).ceil() as usize;
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    let start_idx = start_frame.saturating_mul(channels);
    let end_idx = end_frame.saturating_mul(channels).min(samples.len());
    if start_idx >= end_idx {
        return None;
    }
    let peak = samples[start_idx..end_idx]
        .iter()
        .fold(0.0_f32, |acc, sample| acc.max(sample.abs()));
    Some(peak)
}

fn max_abs_from_peaks(peaks: &WaveformPeaks, start: f32, end: f32) -> Option<f32> {
    let total_frames = peaks.total_frames.max(1);
    let bucket_size = peaks.bucket_size_frames.max(1);
    let start_frame = (start.clamp(0.0, 1.0) * total_frames as f32).floor() as usize;
    let mut end_frame = (end.clamp(0.0, 1.0) * total_frames as f32).ceil() as usize;
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    let end_frame = end_frame.min(total_frames);
    if start_frame >= end_frame {
        return None;
    }
    let start_bucket = start_frame / bucket_size;
    let end_bucket = end_frame.saturating_sub(1) / bucket_size;
    if peaks.mono.is_empty() {
        return None;
    }
    let mut peak = 0.0_f32;
    let last_bucket = peaks.mono.len().saturating_sub(1);
    for bucket in start_bucket..=end_bucket.min(last_bucket) {
        let (min, max) = peaks.mono[bucket];
        peak = peak.max(min.abs().max(max.abs()));
    }
    Some(peak)
}

impl WaveformPeaks {
    /// Sample a subset of columns for the requested viewport.
    pub fn sample_columns_for_view(
        &self,
        view_start: f32,
        view_end: f32,
        width: u32,
        view: WaveformChannelView,
    ) -> WaveformColumnView {
        let width = width.max(1) as usize;
        let total_frames = self.total_frames.max(1);
        let start = view_start.clamp(0.0, 1.0);
        let end = view_end.clamp(start, 1.0);

        let start_frame =
            ((start * total_frames as f32).floor() as usize).min(total_frames.saturating_sub(1));
        let mut end_frame =
            ((end * total_frames as f32).ceil() as usize).clamp(start_frame + 1, total_frames);
        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(total_frames);
        }
        let frames_in_view = end_frame.saturating_sub(start_frame).max(1);

        match view {
            WaveformChannelView::Mono => WaveformColumnView::Mono(self.sample_peak_columns(
                &self.mono,
                start_frame,
                frames_in_view,
                width,
            )),
            WaveformChannelView::SplitStereo => {
                let left_src = self.left.as_ref().unwrap_or(&self.mono);
                let right_src = self.right.as_ref().unwrap_or(&self.mono);
                WaveformColumnView::SplitStereo {
                    left: self.sample_peak_columns(left_src, start_frame, frames_in_view, width),
                    right: self.sample_peak_columns(right_src, start_frame, frames_in_view, width),
                }
            }
        }
    }

    fn sample_peak_columns(
        &self,
        peaks: &[(f32, f32)],
        start_frame: usize,
        frames_in_view: usize,
        width: usize,
    ) -> Vec<(f32, f32)> {
        let bucket_size = self.bucket_size_frames.max(1);
        let bucket_count = peaks.len().max(1);
        let total = frames_in_view as f32;
        let mut columns = vec![(0.0_f32, 0.0_f32); width.max(1)];
        for (x, col) in columns.iter_mut().enumerate() {
            let rel_start = ((x as f32 * total) / width as f32).floor() as usize;
            let rel_end = (((x as f32 + 1.0) * total) / width as f32)
                .ceil()
                .max((rel_start + 1) as f32) as usize;
            let abs_start = start_frame.saturating_add(rel_start);
            let abs_end = start_frame
                .saturating_add(rel_end)
                .min(start_frame.saturating_add(frames_in_view))
                .max(abs_start + 1);
            let start_bucket = (abs_start / bucket_size).min(bucket_count - 1);
            let end_bucket = ((abs_end - 1) / bucket_size)
                .min(bucket_count.saturating_sub(1))
                .max(start_bucket);

            let mut min_v: f32 = 1.0;
            let mut max_v: f32 = -1.0;
            for i in start_bucket..=end_bucket {
                let (lo, hi) = peaks.get(i).copied().unwrap_or((0.0, 0.0));
                min_v = min_v.min(lo);
                max_v = max_v.max(hi);
            }
            if min_v > max_v {
                min_v = 0.0;
                max_v = 0.0;
            }
            *col = (min_v.clamp(-1.0, 1.0), max_v.clamp(-1.0, 1.0));
        }
        columns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn max_abs_in_span_uses_samples() {
        let samples = Arc::from(vec![0.1, -0.2, 0.4, -0.5]);
        let decoded = DecodedWaveform {
            cache_token: 1,
            samples,
            analysis_samples: Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 4,
            channels: 2,
        };
        let peak_full = decoded.max_abs_in_span(0.0, 1.0).unwrap();
        let peak_first_half = decoded.max_abs_in_span(0.0, 0.5).unwrap();
        assert!((peak_full - 0.5).abs() < 1e-6);
        assert!((peak_first_half - 0.2).abs() < 1e-6);
    }

    #[test]
    fn max_abs_in_span_uses_peaks_when_samples_empty() {
        let peaks = WaveformPeaks {
            total_frames: 4,
            channels: 2,
            bucket_size_frames: 2,
            mono: vec![(-0.2, 0.3), (-0.8, 0.6)],
            left: None,
            right: None,
        };
        let decoded = DecodedWaveform {
            cache_token: 2,
            samples: Arc::from(Vec::new()),
            analysis_samples: Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: Some(Arc::new(peaks)),
            duration_seconds: 1.0,
            sample_rate: 4,
            channels: 2,
        };
        let peak_first_bucket = decoded.max_abs_in_span(0.0, 0.5).unwrap();
        let peak_full = decoded.max_abs_in_span(0.0, 1.0).unwrap();
        assert!((peak_first_bucket - 0.3).abs() < 1e-6);
        assert!((peak_full - 0.8).abs() < 1e-6);
    }

    #[test]
    fn peaks_sampling_returns_expected_width() {
        let peaks = WaveformPeaks {
            total_frames: 100,
            channels: 1,
            bucket_size_frames: 10,
            mono: (0..10)
                .map(|i| (-(i as f32) / 10.0, i as f32 / 10.0))
                .collect(),
            left: None,
            right: None,
        };
        let columns = peaks.sample_columns_for_view(0.0, 1.0, 7, WaveformChannelView::Mono);
        let WaveformColumnView::Mono(cols) = columns else {
            panic!("expected mono columns");
        };
        assert_eq!(cols.len(), 7);
        assert!(cols.iter().all(|(min, max)| min <= max));
    }
}
