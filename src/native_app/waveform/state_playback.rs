use std::{fs::File, io::BufReader};

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
        } else {
            let cache_file = self.file.playback_cache_file.as_ref()?;
            NormalizedAuditionGainSourceKey::CacheFile {
                path: cache_file.path.clone(),
                sample_count: cache_file.sample_count,
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
        let cache_file = self.file.playback_cache_file.as_ref()?;
        let sample_count = usize::try_from(cache_file.sample_count).ok()?;
        let file = File::open(&cache_file.path).ok()?;
        let mut reader = BufReader::new(file);
        wavecrate::audio::peak_for_interleaved_f32_reader_span(
            &mut reader,
            sample_count,
            self.file.channels,
            start,
            end,
        )
        .map(wavecrate::audio::normalized_gain_from_peak)
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
