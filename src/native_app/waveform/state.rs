use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use wavecrate::selection::SelectionRange;

use super::{WaveformDrag, WaveformFile, WaveformViewport, similar_sections::SimilarSectionsState};

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
