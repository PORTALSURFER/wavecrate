//! Retained browser/status text payloads keyed by layout and model fingerprints.

mod browser;
mod keys;
mod status;

use super::*;
use browser::build_browser_segment_text_cache;
use keys::{browser_segment_text_cache_key, status_bar_text_cache_key};
use status::build_status_bar_text_cache;
use std::sync::Arc;

impl NativeShellState {
    /// Resolve cached browser-segment text/layout payloads for the current frame.
    pub(super) fn cached_browser_segment_text(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> Arc<BrowserSegmentTextCacheValue> {
        self.browser_segment_text_frame_counts.lookup_count = self
            .browser_segment_text_frame_counts
            .lookup_count
            .saturating_add(1);
        let key = browser_segment_text_cache_key(layout, style, model);
        if self.browser_segment_text_cache_key != Some(key) {
            self.browser_segment_text_cache = Some(Arc::new(build_browser_segment_text_cache(
                layout, style, model,
            )));
            self.browser_segment_text_cache_key = Some(key);
            self.browser_segment_text_frame_counts.cache_miss_count = self
                .browser_segment_text_frame_counts
                .cache_miss_count
                .saturating_add(1);
        } else {
            self.browser_segment_text_frame_counts.cache_hit_count = self
                .browser_segment_text_frame_counts
                .cache_hit_count
                .saturating_add(1);
        }
        self.browser_segment_text_cache
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| Arc::new(build_browser_segment_text_cache(layout, style, model)))
    }

    /// Resolve cached status-bar text/layout payloads for the current frame.
    pub(super) fn cached_status_bar_text(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> Arc<StatusBarTextCacheValue> {
        self.status_bar_text_frame_counts.lookup_count = self
            .status_bar_text_frame_counts
            .lookup_count
            .saturating_add(1);
        let key = status_bar_text_cache_key(
            layout,
            style,
            model,
            self.transport_running,
            self.selected_column,
        );
        if self.status_bar_text_cache_key != Some(key) {
            self.status_bar_text_cache = Some(Arc::new(build_status_bar_text_cache(
                layout,
                style,
                model,
                self.transport_running,
                self.selected_column,
            )));
            self.status_bar_text_cache_key = Some(key);
            self.status_bar_text_frame_counts.cache_miss_count = self
                .status_bar_text_frame_counts
                .cache_miss_count
                .saturating_add(1);
        } else {
            self.status_bar_text_frame_counts.cache_hit_count = self
                .status_bar_text_frame_counts
                .cache_hit_count
                .saturating_add(1);
        }
        self.status_bar_text_cache
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| {
                Arc::new(build_status_bar_text_cache(
                    layout,
                    style,
                    model,
                    self.transport_running,
                    self.selected_column,
                ))
            })
    }

    /// Return the latest browser-segment text-cache lookup counts in tests.
    #[cfg(test)]
    pub(crate) fn browser_segment_text_frame_counts(&self) -> SegmentTextCacheFrameCounts {
        self.browser_segment_text_frame_counts
    }

    /// Return the latest status-bar text-cache lookup counts in tests.
    #[cfg(test)]
    pub(crate) fn status_bar_text_frame_counts(&self) -> SegmentTextCacheFrameCounts {
        self.status_bar_text_frame_counts
    }
}
