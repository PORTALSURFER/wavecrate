use super::{
    WaveformState,
    state::{NormalizedAuditionGainCacheKey, NormalizedAuditionGainSourceKey},
};

impl WaveformState {
    pub(in crate::native_app) fn normalized_audition_gain_for_span(
        &self,
        start: f32,
        end: f32,
    ) -> f32 {
        let Some(key) = self.normalized_audition_gain_cache_key(start, end) else {
            return 1.0;
        };
        self.normalized_audition_gain_cache
            .get_or_compute(key, || {
                self.compute_normalized_audition_gain_for_span(start, end)
            })
            .unwrap_or(1.0)
    }

    pub(in crate::native_app) fn normalized_audition_preview_selection(
        &self,
    ) -> wavecrate::selection::SelectionRange {
        self.play_selection
            .filter(|selection| selection.width() > 0.0)
            .unwrap_or_else(|| wavecrate::selection::SelectionRange::new(0.0, 1.0))
    }

    fn normalized_audition_gain_cache_key(
        &self,
        start: f32,
        end: f32,
    ) -> Option<NormalizedAuditionGainCacheKey> {
        let source = if let Some(samples) = self.file.playback_samples.as_ref() {
            NormalizedAuditionGainSourceKey::Samples {
                ptr: samples.as_ptr() as usize,
                sample_count: samples.len(),
            }
        } else if let Some(cache_file) = self.file.playback_cache_file.as_ref() {
            NormalizedAuditionGainSourceKey::CacheFile {
                path: cache_file.path.clone(),
                sample_count: cache_file.sample_count,
            }
        } else {
            NormalizedAuditionGainSourceKey::Summary {
                path: self.file.path.clone(),
                content_revision: self.file.content_revision,
                frames: self.file.frames,
            }
        };
        Some(NormalizedAuditionGainCacheKey {
            source,
            channels: self.file.channels.max(1),
            start_bits: start.to_bits(),
            end_bits: end.to_bits(),
        })
    }

    fn compute_normalized_audition_gain_for_span(&self, start: f32, end: f32) -> Option<f32> {
        if let Some(samples) = self.file.playback_samples.as_ref() {
            return wavecrate::audio::peak_for_interleaved_span(
                samples,
                self.file.channels,
                start,
                end,
            )
            .map(wavecrate::audio::normalized_gain_from_peak);
        }
        self.normalized_audition_summary_peak_for_span(start, end)
            .map(wavecrate::audio::normalized_gain_from_peak)
    }

    fn normalized_audition_summary_peak_for_span(&self, start: f32, end: f32) -> Option<f32> {
        const RAW_BAND: usize = 3;

        let summary = self.file.gpu_signal_summary.as_ref();
        if summary.frames == 0 || summary.band_count <= RAW_BAND {
            return None;
        }
        let level = summary.levels.first()?;
        let bucket_count = level.buckets.len() / summary.band_count;
        if bucket_count == 0 {
            return None;
        }
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let start_frame = (start.clamp(0.0, 1.0) * summary.frames as f32).floor() as usize;
        let mut end_frame = (end.clamp(0.0, 1.0) * summary.frames as f32).ceil() as usize;
        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(summary.frames);
        }
        let bucket_frames = level.bucket_frames.max(1);
        let first_bucket = start_frame / bucket_frames;
        let last_bucket = end_frame
            .saturating_sub(1)
            .checked_div(bucket_frames)?
            .min(bucket_count.saturating_sub(1));
        let mut peak = 0.0_f32;
        for bucket in first_bucket..=last_bucket {
            let raw = level.buckets.get(
                bucket
                    .saturating_mul(summary.band_count)
                    .saturating_add(RAW_BAND),
            )?;
            peak = peak.max(raw.min.abs()).max(raw.max.abs());
        }
        peak.is_finite().then_some(peak)
    }

    pub(in crate::native_app) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(in crate::native_app) fn playback_visual_generation(&self) -> u64 {
        self.playback_visual_generation
    }

    pub(in crate::native_app) fn playhead_ratio(&self) -> Option<f32> {
        self.playhead_ratio
    }

    pub(in crate::native_app) fn play_mark_ratio(&self) -> Option<f32> {
        self.play_mark_ratio
    }

    pub(in crate::native_app) fn take_pending_playback_start(&mut self) -> Option<f32> {
        self.pending_playback_start.take()
    }

    pub(in crate::native_app) fn take_pending_sample_slide_frame_offset(&mut self) -> Option<i64> {
        self.pending_sample_slide_frame_offset.take()
    }

    pub(in crate::native_app) fn start_playback(&mut self, ratio: f32) {
        self.start_playback_with_marker(ratio, true);
    }

    pub(in crate::native_app) fn start_playback_without_marker(&mut self, ratio: f32) {
        self.start_playback_with_marker(ratio, false);
    }

    fn start_playback_with_marker(&mut self, ratio: f32, show_marker: bool) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playing = true;
        self.playback_visual_generation = self.playback_visual_generation.wrapping_add(1);
        self.play_mark_ratio = show_marker.then_some(ratio);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(in crate::native_app) fn set_playhead_ratio(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(in crate::native_app) fn stop_playback(&mut self) {
        if self.playing || self.playhead_ratio.is_some() {
            self.playback_visual_generation = self.playback_visual_generation.wrapping_add(1);
        }
        self.playing = false;
        self.playhead_ratio = None;
    }
}
