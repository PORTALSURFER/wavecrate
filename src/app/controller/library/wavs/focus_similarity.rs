use super::*;
use std::path::PathBuf;
use std::time::Instant;

impl AppController {
    /// Reset all waveform and playback visuals.
    pub(crate) fn clear_waveform_view(&mut self) {
        waveform_view::clear_waveform_view(self);
    }

    /// Clear near-duplicate highlights for the focused sample.
    pub(crate) fn clear_focused_similarity_highlight(&mut self) {
        self.runtime.pending_similarity_refresh = None;
        self.runtime.pending_similarity_refresh_not_before = None;
        self.ui.browser.search.focused_similarity = None;
    }

    /// Refresh near-duplicate highlights for the focused sample.
    pub(crate) fn refresh_focused_similarity_highlight(
        &mut self,
        sample_id: &str,
        anchor_index: Option<usize>,
    ) {
        self.ui.browser.search.focused_similarity =
            similar::build_focused_similarity_highlight(self, sample_id, anchor_index)
                .unwrap_or_default();
    }

    /// Queue a focused-similarity highlight refresh for frame-time execution.
    pub(crate) fn defer_focused_similarity_highlight_refresh(
        &mut self,
        sample_id: String,
        relative_path: PathBuf,
        anchor_index: Option<usize>,
    ) {
        self.runtime.pending_similarity_refresh = Some(
            crate::app::controller::state::runtime::PendingFocusedSimilarityRefresh {
                sample_id,
                relative_path,
                anchor_index,
            },
        );
        self.runtime.pending_similarity_refresh_not_before =
            Some(Instant::now() + FOCUSED_SIMILARITY_REFRESH_DEBOUNCE);
    }

    /// Flush any queued focused-similarity refresh request.
    pub(crate) fn flush_pending_focused_similarity_highlight_refresh(&mut self) {
        if self
            .runtime
            .pending_similarity_refresh_not_before
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        self.runtime.pending_similarity_refresh_not_before = None;
        let Some(pending) = self.runtime.pending_similarity_refresh.take() else {
            return;
        };
        if self.sample_view.wav.selected_wav.as_deref() != Some(pending.relative_path.as_path()) {
            return;
        }
        self.refresh_focused_similarity_highlight(&pending.sample_id, pending.anchor_index);
    }

    /// Return true when a focused-similarity refresh is queued.
    pub(crate) fn has_pending_focused_similarity_highlight_refresh(&self) -> bool {
        self.runtime.pending_similarity_refresh.is_some()
    }
}
