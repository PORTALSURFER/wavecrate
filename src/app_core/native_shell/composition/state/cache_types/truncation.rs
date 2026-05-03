use super::*;

/// Per-build browser-row truncation cache lookup counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BrowserRowTruncationFrameCounts {
    /// Number of truncation lookups requested while building browser rows.
    pub lookup_count: u32,
    /// Number of lookups that reused cached truncated strings.
    pub cache_hit_count: u32,
    /// Number of lookups that required fresh truncation work.
    pub cache_miss_count: u32,
}

/// Browser row text variants tracked in truncation cache keys.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) enum BrowserRowTextKind {
    /// Primary item label text in browser rows.
    Item,
    /// Secondary inline metadata text in browser rows.
    Bucket,
}

/// Lookup key for one browser-row truncation output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRowTruncationEntryKey {
    /// Stable visible-row identity used to scope cached text.
    pub row_id: u32,
    /// Quantized width bucket used by truncation heuristics.
    pub width_bucket: u16,
    /// Quantized font-size bucket used by truncation heuristics.
    pub font_size_bucket: u16,
    /// Distinguishes item-label vs bucket-label truncation outputs.
    pub text_kind: BrowserRowTextKind,
}

/// Invalidation key for browser-row truncation cache content.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserRowTruncationCacheKey {
    /// Browser rows region minimum x-coordinate.
    pub browser_rows_min_x: u32,
    /// Browser rows region minimum y-coordinate.
    pub browser_rows_min_y: u32,
    /// Browser rows region maximum x-coordinate.
    pub browser_rows_max_x: u32,
    /// Browser rows region maximum y-coordinate.
    pub browser_rows_max_y: u32,
    /// Item-label font size token bits.
    pub font_body_bits: u32,
    /// Bucket-label font size token bits.
    pub font_meta_bits: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Visible-window row-label content revision fingerprint.
    pub row_text_revision: u64,
}

/// Invalidation key for browser action/button hit-test geometry caches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserActionHitTestCacheKey {
    /// Browser toolbar region minimum x-coordinate.
    pub browser_toolbar_min_x: u32,
    /// Browser toolbar region minimum y-coordinate.
    pub browser_toolbar_min_y: u32,
    /// Browser toolbar region maximum x-coordinate.
    pub browser_toolbar_max_x: u32,
    /// Browser toolbar region maximum y-coordinate.
    pub browser_toolbar_max_y: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Stable digest of action-strip and triage-chip model fields.
    pub model_signature: u64,
}

/// Invalidation key for waveform-toolbar hit-test geometry caches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct WaveformToolbarHitTestCacheKey {
    /// Waveform header region minimum x-coordinate.
    pub waveform_header_min_x: u32,
    /// Waveform header region minimum y-coordinate.
    pub waveform_header_min_y: u32,
    /// Waveform header region maximum x-coordinate.
    pub waveform_header_max_x: u32,
    /// Waveform header region maximum y-coordinate.
    pub waveform_header_max_y: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Packed waveform-toolbar model state flags.
    pub model_flags: u16,
    /// Stable digest of waveform tempo label text.
    pub tempo_label_signature: u64,
    /// Stable digest of the loaded waveform label used by action availability.
    pub loaded_label_signature: u64,
    /// Whether waveform data is still loading.
    pub waveform_loading: bool,
    /// Whether waveform BPM editor mode is active.
    pub bpm_editor_active: bool,
    /// Stable digest of waveform BPM editor display text.
    pub bpm_editor_display_signature: u64,
    /// Current waveform slice preview count, used to invalidate cleanup actions.
    pub waveform_slice_count: u32,
}

/// Small retained LRU cache for browser-row text truncation outputs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::gui::native_shell::state) struct BrowserRowTruncationCache {
    pub values: HashMap<BrowserRowTruncationEntryKey, BrowserRowTruncationCacheValue>,
    pub touch_epoch: u64,
}

/// One cached truncation result with the latest logical access epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui::native_shell::state) struct BrowserRowTruncationCacheValue {
    pub truncated: String,
    pub last_touch_epoch: u64,
}

impl BrowserRowTruncationCache {
    /// Clear all retained truncation entries.
    pub(in crate::gui::native_shell::state) fn clear(&mut self) {
        self.values.clear();
        self.touch_epoch = 0;
    }

    /// Resolve one truncation output from cache or compute and insert on miss.
    pub(in crate::gui::native_shell::state) fn resolve(
        &mut self,
        key: BrowserRowTruncationEntryKey,
        text: &str,
        max_width: f32,
        font_size: f32,
        frame_counts: &mut BrowserRowTruncationFrameCounts,
    ) -> String {
        let touch_epoch = self.next_touch_epoch();
        frame_counts.lookup_count = frame_counts.lookup_count.saturating_add(1);
        if let Some(cached) = self.values.get_mut(&key) {
            frame_counts.cache_hit_count = frame_counts.cache_hit_count.saturating_add(1);
            cached.last_touch_epoch = touch_epoch;
            return cached.truncated.clone();
        }
        frame_counts.cache_miss_count = frame_counts.cache_miss_count.saturating_add(1);
        let truncated = truncate_to_width(text, max_width, font_size);
        self.insert(key, truncated.clone(), touch_epoch);
        truncated
    }

    /// Return the next logical access epoch used for cache aging.
    fn next_touch_epoch(&mut self) -> u64 {
        if self.touch_epoch == u64::MAX {
            self.clear();
        }
        self.touch_epoch = self.touch_epoch.saturating_add(1);
        self.touch_epoch
    }

    /// Insert one key/value pair and enforce the fixed cache capacity via LRU epoch eviction.
    fn insert(&mut self, key: BrowserRowTruncationEntryKey, value: String, touch_epoch: u64) {
        self.values.insert(
            key,
            BrowserRowTruncationCacheValue {
                truncated: value,
                last_touch_epoch: touch_epoch,
            },
        );
        while self.values.len() > BROWSER_ROW_TRUNCATION_CACHE_CAPACITY {
            let Some((evicted, _)) = self
                .values
                .iter()
                .min_by_key(|(_, value)| value.last_touch_epoch)
                .map(|(key, value)| (*key, value.last_touch_epoch))
            else {
                break;
            };
            self.values.remove(&evicted);
        }
    }
}
