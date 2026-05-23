use crate::app::controller::FeatureCacheKey;
use crate::sample_sources::{Rating, SourceId, WavEntry};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Compact synchronous browser-filter metadata aligned to absolute wav-entry indices.
#[derive(Clone)]
pub(super) struct CompactBrowserEntry {
    pub(super) relative_path: PathBuf,
    pub(super) tag: Rating,
    pub(super) looped: bool,
    pub(super) locked: bool,
    pub(super) missing: bool,
    pub(super) last_played_at: Option<i64>,
    pub(super) tag_named: bool,
}

impl CompactBrowserEntry {
    /// Build the compact retained entry payload required by the sync browser pipeline.
    pub(super) fn from_wav_entry(entry: WavEntry) -> Self {
        Self {
            relative_path: entry.relative_path,
            tag: entry.tag,
            looped: entry.looped,
            locked: entry.locked,
            missing: entry.missing,
            last_played_at: entry.last_played_at,
            tag_named: entry.tag_named,
        }
    }
}

/// Borrowed browser-row metadata used by page-load-free projection helpers.
#[derive(Clone, Copy)]
pub(crate) struct BrowserProjectionEntryRef<'a> {
    /// Stable source-relative path for labels and cached metadata.
    pub(crate) relative_path: &'a Path,
    /// Current triage rating for the sample.
    pub(crate) tag: Rating,
    /// Whether the sample should render the loop badge.
    pub(crate) looped: bool,
    /// Whether the sample is locked as a keep.
    pub(crate) locked: bool,
    /// Whether the sample is missing on disk.
    pub(crate) missing: bool,
    /// Last-played timestamp used for playback-age buckets.
    pub(crate) last_played_at: Option<i64>,
    /// Whether the sample filename is known to be tag-derived.
    pub(crate) tag_named: bool,
}

impl<'a> BrowserProjectionEntryRef<'a> {
    /// Borrow one projection entry from the retained compact browser snapshot.
    pub(super) fn from_compact_entry(entry: &'a CompactBrowserEntry) -> Self {
        Self {
            relative_path: entry.relative_path.as_path(),
            tag: entry.tag,
            looped: entry.looped,
            locked: entry.locked,
            missing: entry.missing,
            last_played_at: entry.last_played_at,
            tag_named: entry.tag_named,
        }
    }

    /// Borrow one projection entry from an already loaded wav-entry page.
    pub(super) fn from_loaded_entry(entry: &'a WavEntry) -> Self {
        Self {
            relative_path: entry.relative_path.as_path(),
            tag: entry.tag,
            looped: entry.looped,
            locked: entry.locked,
            missing: entry.missing,
            last_played_at: entry.last_played_at,
            tag_named: entry.tag_named,
        }
    }
}

/// Retained ordered path snapshot used to refresh browser feature metadata.
#[derive(Clone)]
pub(crate) struct BrowserFeatureCacheSnapshot {
    /// Stable key for the current ordered path list.
    pub(crate) key: FeatureCacheKey,
    /// Ordered relative paths aligned to absolute wav-entry indices.
    pub(crate) entry_paths: Arc<[PathBuf]>,
}

/// Stable identity for the stage-A base snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct BaseStageFingerprint {
    pub(super) source_id: Option<SourceId>,
    pub(super) source_revision: Option<u64>,
    pub(super) entries_len: usize,
}

/// Cached next playback-age rollover token for one retained base snapshot and chip set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PlaybackAgeTokenCache {
    pub(super) base_fingerprint_hash: u64,
    pub(super) filter_hash: u64,
    pub(super) token: Option<i64>,
}
