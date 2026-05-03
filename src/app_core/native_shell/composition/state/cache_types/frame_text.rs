use super::*;

/// Per-build lookup counts for retained browser/status text payloads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SegmentTextCacheFrameCounts {
    /// Number of retained cache lookups issued during one segment build.
    pub lookup_count: u32,
    /// Number of lookups that reused the retained payload.
    pub cache_hit_count: u32,
    /// Number of lookups that rebuilt the retained payload.
    pub cache_miss_count: u32,
}

/// Invalidation key for retained browser-frame/tab/footer text payloads.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserSegmentTextCacheKey {
    /// Browser tabs region minimum x-coordinate.
    pub browser_tabs_min_x: u32,
    /// Browser tabs region minimum y-coordinate.
    pub browser_tabs_min_y: u32,
    /// Browser tabs region maximum x-coordinate.
    pub browser_tabs_max_x: u32,
    /// Browser tabs region maximum y-coordinate.
    pub browser_tabs_max_y: u32,
    /// Browser toolbar region minimum x-coordinate.
    pub browser_toolbar_min_x: u32,
    /// Browser toolbar region minimum y-coordinate.
    pub browser_toolbar_min_y: u32,
    /// Browser toolbar region maximum x-coordinate.
    pub browser_toolbar_max_x: u32,
    /// Browser toolbar region maximum y-coordinate.
    pub browser_toolbar_max_y: u32,
    /// Browser footer region minimum x-coordinate.
    pub browser_footer_min_x: u32,
    /// Browser footer mininum y-coordinate.
    pub browser_footer_min_y: u32,
    /// Browser footer region maximum x-coordinate.
    pub browser_footer_max_x: u32,
    /// Browser footer region maximum y-coordinate.
    pub browser_footer_max_y: u32,
    /// Meta-label font size token bits.
    pub font_meta_bits: u32,
    /// Header-label font size token bits.
    pub font_header_bits: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Stable digest of browser-frame/tab/footer text inputs.
    pub model_signature: u64,
}

/// Retained text/layout payload for browser tabs, toolbar labels, and footer copy.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct BrowserSegmentTextCacheValue {
    /// Precomputed browser-tab label rects.
    pub tabs_text_layout: BrowserTabsTextLayout,
    /// Precomputed toolbar label rects.
    pub toolbar_text_layout: BrowserToolbarTextLayout,
    /// Precomputed footer label rect.
    pub footer_text_rect: Rect,
    /// Final items-tab label.
    pub items_tab_label: String,
    /// Final map-tab label.
    pub map_tab_label: String,
    /// Final browser-search label.
    pub search_label: String,
    /// Final activity-chip label.
    pub activity_label: String,
    /// Final sort-chip label.
    pub sort_label: String,
    /// Final browser footer summary label.
    pub footer_label: String,
}

/// Invalidation key for retained status-bar text payloads.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::gui::native_shell::state) struct StatusBarTextCacheKey {
    /// Status left-segment region minimum x-coordinate.
    pub status_left_min_x: u32,
    /// Status left-segment region minimum y-coordinate.
    pub status_left_min_y: u32,
    /// Status left-segment region maximum x-coordinate.
    pub status_left_max_x: u32,
    /// Status left-segment region maximum y-coordinate.
    pub status_left_max_y: u32,
    /// Status center-segment region minimum x-coordinate.
    pub status_center_min_x: u32,
    /// Status center-segment region minimum y-coordinate.
    pub status_center_min_y: u32,
    /// Status center-segment region maximum x-coordinate.
    pub status_center_max_x: u32,
    /// Status center-segment region maximum y-coordinate.
    pub status_center_max_y: u32,
    /// Status right-segment region minimum x-coordinate.
    pub status_right_min_x: u32,
    /// Status right-segment region minimum y-coordinate.
    pub status_right_min_y: u32,
    /// Status right-segment region maximum x-coordinate.
    pub status_right_max_x: u32,
    /// Status right-segment region maximum y-coordinate.
    pub status_right_max_y: u32,
    /// Status progress-segment region minimum x-coordinate.
    pub status_progress_min_x: u32,
    /// Status progress-segment region minimum y-coordinate.
    pub status_progress_min_y: u32,
    /// Status progress-segment region maximum x-coordinate.
    pub status_progress_max_x: u32,
    /// Status progress-segment region maximum y-coordinate.
    pub status_progress_max_y: u32,
    /// Status-font size token bits.
    pub font_status_bits: u32,
    /// Effective UI scale token bits.
    pub ui_scale: u32,
    /// Whether transport playback is currently active.
    pub transport_running: bool,
    /// Stable digest of footer text inputs.
    pub model_signature: u64,
}

/// Retained text/layout payload for the status bar and inline progress copy.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui::native_shell::state) struct StatusBarTextCacheValue {
    /// Precomputed left status-label rect.
    pub left_text_rect: Rect,
    /// Precomputed center status-label rect.
    pub center_text_rect: Rect,
    /// Precomputed right status-label rect.
    pub right_text_rect: Rect,
    /// Precomputed progress-slot counter rect.
    pub progress_text_rect: Rect,
    /// Precomputed progress-track canvas rect.
    pub progress_track_rect: Rect,
    /// Final left-side footer label.
    pub left_label: String,
    /// Final center footer label when inline progress is inactive.
    pub center_label: String,
    /// Final right-side footer label.
    pub right_label: String,
    /// Final inline progress label.
    pub progress_label: String,
    /// Final progress-slot counter.
    pub progress_counter: String,
    /// Whether inline progress copy replaces the center label.
    pub inline_progress_active: bool,
}
