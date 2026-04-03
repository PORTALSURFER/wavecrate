use super::*;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::waveform::DecodedWaveform;
use std::time::Duration;

mod audio_loading;
mod browser_actions;
/// High-level browser/search/selection facade methods on `AppController`.
mod browser_facade;
mod browser_history;
mod browser_lists;
mod browser_marks;
/// Staged browser row pipeline caching and deterministic recompute helpers.
mod browser_pipeline;
mod browser_search;
pub(crate) mod browser_search_worker;
mod browser_viewport;
mod duplicate_cleanup;
/// Lightweight entry/cache paging and lookup facade methods.
mod entry_access;
/// File rename/normalize/update mutation helpers for wav browser state.
mod entry_mutation;
#[cfg(test)]
mod entry_mutation_tests;
mod feature_cache;
/// Focused-similarity refresh and waveform view reset helpers.
mod focus_similarity;
/// Source DB and in-memory metadata lookup/cache helpers.
mod metadata_cache;
/// Async metadata persistence helpers for tag, loop, BPM, and playback-age writes.
mod metadata_async;
/// Metadata and entry-mutation facade methods on `AppController`.
mod metadata_facade;
/// Shared fuzzy-search scoring and cache-reuse helpers for sync and worker paths.
mod search_scoring;
mod selection_ops;
mod similar;
mod waveform_loading;
pub mod waveform_rendering;

mod waveform_view;

pub(crate) use browser_actions::{BrowserReviewFollowUpPlan, BrowserReviewLinearMode};
pub(crate) use browser_pipeline::BrowserPipelineCache;
pub(crate) use browser_search::BrowserSearchCache;
#[cfg(test)]
pub(crate) use browser_search::with_browser_async_pipeline_enabled_for_tests;
pub(crate) use similar::{
    apply_pending_similarity_filter_rebuild, cancel_pending_similarity_filter_rebuild,
    schedule_similarity_filter_rebuild_after_delete_with_state,
};
pub(crate) use waveform_loading::FinishWaveformLoadShared;
pub(crate) use waveform_rendering::WaveformRenderMeta;

/// Upper bound for waveform texture width to stay within GPU limits.
pub(crate) const MAX_TEXTURE_WIDTH: u32 = 16_384;
/// Debounce duration for expensive focused-similarity highlight recomputes.
const FOCUSED_SIMILARITY_REFRESH_DEBOUNCE: Duration = Duration::from_millis(160);
