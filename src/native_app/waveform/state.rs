use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use wavecrate::selection::SelectionRange;

use super::{WaveformDrag, WaveformFile, WaveformViewport, similar_sections::SimilarSectionsState};
use radiant::runtime::GpuSignalSummary;

const DETAIL_MAX_BUCKETS: usize = 65_536;
const DETAIL_MAX_VISIBLE_FRAMES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct WaveformDetailKey {
    pub path: PathBuf,
    pub content_revision: u64,
    pub start_frame: usize,
    pub end_frame: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformDetailResult {
    pub key: WaveformDetailKey,
    pub summary: Result<Arc<GpuSignalSummary>, String>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app::waveform) struct WaveformDetailSummary {
    pub key: WaveformDetailKey,
    pub summary: Arc<GpuSignalSummary>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformState {
    pub(in crate::native_app::waveform) file: Arc<WaveformFile>,
    pub(in crate::native_app::waveform) viewport: WaveformViewport,
    pub(in crate::native_app::waveform) zoom_anchor_ratio: f32,
    pub(in crate::native_app::waveform) playing: bool,
    pub(in crate::native_app::waveform) playback_visual_generation: u64,
    pub(in crate::native_app::waveform) playhead_ratio: Option<f32>,
    pub(in crate::native_app::waveform) play_mark_ratio: Option<f32>,
    pub(in crate::native_app::waveform) edit_mark_ratio: Option<f32>,
    pub(in crate::native_app::waveform) play_selection: Option<SelectionRange>,
    pub(in crate::native_app::waveform) edit_selection: Option<SelectionRange>,
    pub(in crate::native_app::waveform) zero_crossing_snap_enabled: bool,
    pub(in crate::native_app::waveform) marked_play_ranges: Vec<SelectionRange>,
    pub(in crate::native_app::waveform) extracted_ranges: Vec<SelectionRange>,
    pub(in crate::native_app::waveform) similar_sections: SimilarSectionsState,
    pub(in crate::native_app::waveform) play_selection_flash_frames: u8,
    pub(in crate::native_app::waveform) edit_selection_flash_frames: u8,
    pub(in crate::native_app::waveform) play_selection_denied_flash_frames: u8,
    pub(in crate::native_app::waveform) edit_selection_denied_flash_frames: u8,
    pub(in crate::native_app::waveform) copy_flash_frames: u8,
    pub(in crate::native_app::waveform) protected_source_error_flash_frames: u8,
    pub(in crate::native_app::waveform) active_drag: Option<WaveformDrag>,
    pub(in crate::native_app::waveform) pending_playback_start: Option<f32>,
    pub(in crate::native_app::waveform) pending_sample_slide_frame_offset: Option<i64>,
    pub(in crate::native_app::waveform) normalized_audition_gain_cache: NormalizedAuditionGainCache,
    pub(in crate::native_app::waveform) detail_summary: Option<WaveformDetailSummary>,
    pub(in crate::native_app::waveform) pending_detail_key: Option<WaveformDetailKey>,
    pub(in crate::native_app::waveform) failed_detail_key: Option<WaveformDetailKey>,
}

impl WaveformState {
    pub(in crate::native_app) fn desired_detail_key(&self) -> Option<WaveformDetailKey> {
        if self.pending_detail_key.is_some() || !super::audio_file::is_wav_path(&self.file.path) {
            return None;
        }
        let range = self
            .viewport
            .clamped_index_viewport(self.file.frames, super::MIN_VISIBLE_FRAMES);
        let visible = range.end.saturating_sub(range.start);
        if visible == 0 || visible > DETAIL_MAX_VISIBLE_FRAMES {
            return None;
        }
        let overview_bucket_frames = self.file.gpu_signal_summary.levels.first()?.bucket_frames;
        let target_bucket_frames = visible.div_ceil(DETAIL_MAX_BUCKETS).max(1);
        if overview_bucket_frames <= target_bucket_frames {
            return None;
        }
        let key = WaveformDetailKey {
            path: self.file.path.clone(),
            content_revision: self.file.content_revision(),
            start_frame: range.start,
            end_frame: range.end,
        };
        if self
            .detail_summary
            .as_ref()
            .is_some_and(|detail| detail.key == key)
        {
            return None;
        }
        if self.failed_detail_key.as_ref() == Some(&key) {
            return None;
        }
        Some(key)
    }

    pub(in crate::native_app) fn mark_detail_pending(&mut self, key: WaveformDetailKey) {
        self.pending_detail_key = Some(key);
    }

    pub(in crate::native_app) fn apply_detail_result(&mut self, result: WaveformDetailResult) {
        if self.pending_detail_key.as_ref() != Some(&result.key) {
            return;
        }
        self.pending_detail_key = None;
        let current = WaveformDetailKey {
            path: self.file.path.clone(),
            content_revision: self.file.content_revision(),
            start_frame: self
                .viewport
                .clamped_index_viewport(self.file.frames, super::MIN_VISIBLE_FRAMES)
                .start,
            end_frame: self
                .viewport
                .clamped_index_viewport(self.file.frames, super::MIN_VISIBLE_FRAMES)
                .end,
        };
        if current != result.key {
            return;
        }
        match result.summary {
            Ok(summary) => {
                self.failed_detail_key = None;
                self.detail_summary = Some(WaveformDetailSummary {
                    key: result.key,
                    summary,
                });
            }
            Err(_) => self.failed_detail_key = Some(result.key),
        }
    }

    pub(in crate::native_app::waveform) fn render_detail(&self) -> Option<&WaveformDetailSummary> {
        self.detail_summary.as_ref().filter(|detail| {
            detail.key.path == self.file.path
                && detail.key.content_revision == self.file.content_revision()
                && detail.key.start_frame
                    == self
                        .viewport
                        .clamped_index_viewport(self.file.frames, super::MIN_VISIBLE_FRAMES)
                        .start
                && detail.key.end_frame
                    == self
                        .viewport
                        .clamped_index_viewport(self.file.frames, super::MIN_VISIBLE_FRAMES)
                        .end
        })
    }
}

#[derive(Clone, Debug, Default)]
pub(in crate::native_app::waveform) struct NormalizedAuditionGainCache {
    inner: Arc<Mutex<Option<NormalizedAuditionGainCacheEntry>>>,
}

#[derive(Clone, Debug)]
struct NormalizedAuditionGainCacheEntry {
    key: NormalizedAuditionGainCacheKey,
    gain: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app::waveform) struct NormalizedAuditionGainCacheKey {
    pub(in crate::native_app::waveform) source: NormalizedAuditionGainSourceKey,
    pub(in crate::native_app::waveform) channels: usize,
    pub(in crate::native_app::waveform) start_bits: u32,
    pub(in crate::native_app::waveform) end_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app::waveform) enum NormalizedAuditionGainSourceKey {
    Samples {
        ptr: usize,
        sample_count: usize,
    },
    CacheFile {
        path: PathBuf,
        sample_count: u64,
    },
    Summary {
        path: PathBuf,
        content_revision: u64,
        frames: usize,
    },
}

impl NormalizedAuditionGainCache {
    pub(in crate::native_app::waveform) fn get_or_compute(
        &self,
        key: NormalizedAuditionGainCacheKey,
        compute: impl FnOnce() -> Option<f32>,
    ) -> Option<f32> {
        if let Ok(guard) = self.inner.lock()
            && let Some(entry) = guard.as_ref()
            && entry.key == key
        {
            return Some(entry.gain);
        }
        let gain = compute()?;
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(NormalizedAuditionGainCacheEntry { key, gain });
        }
        Some(gain)
    }
}
