use super::selection_transition_machine::{
    BrowserSelectionCandidate, BrowserSelectionTransitionEffect, BrowserSelectionTransitionMachine,
    BrowserSelectionTransitionOutcome,
};
use super::*;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::runtime::{
    BrowserSelectionCommitRequest, BrowserSelectionCommitStage, BrowserSelectionLoadState,
    BrowserSelectionTransition,
};

impl BrowserSelectionTransition {
    /// Return true when the candidate sample still needs a deferred commit flush.
    pub(crate) fn commit_flush_pending(&self) -> bool {
        matches!(
            self.commit_stage,
            BrowserSelectionCommitStage::DispatchPending(_)
        )
    }

    /// Return true when the candidate still owns browser preview-only focus.
    pub(crate) fn preview_pending(&self) -> bool {
        matches!(self.commit_stage, BrowserSelectionCommitStage::Preview)
    }

    /// Return true when the candidate still owns any pending or active load work.
    pub(crate) fn load_in_flight(&self) -> bool {
        self.load_requested() || self.load_state != BrowserSelectionLoadState::Stable
    }

    /// Return true when the candidate should still hand off into loading.
    pub(crate) fn load_requested(&self) -> bool {
        matches!(
            self.commit_stage,
            BrowserSelectionCommitStage::DispatchPending(BrowserSelectionCommitRequest {
                load_requested: true,
                ..
            })
        )
    }
}

impl AppController {
    /// Return true when one browser-selection commit still needs frame-time dispatch.
    pub(crate) fn has_pending_browser_focus_commit(&self) -> bool {
        self.runtime
            .browser
            .selection_transition
            .as_ref()
            .is_some_and(BrowserSelectionTransition::commit_flush_pending)
    }

    /// Publish the newest browser-selection candidate, superseding stale transition work.
    pub(crate) fn replace_browser_selection_transition(
        &mut self,
        next: BrowserSelectionTransition,
    ) {
        self.reduce_browser_selection_transition(|machine| machine.replace_transition(next));
    }

    /// Update the controller-owned transition after browser focus moves to a preview candidate.
    pub(crate) fn publish_browser_preview_selection(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        entry_index: usize,
    ) {
        let candidate = BrowserSelectionCandidate::new(source_id, relative_path, entry_index);
        self.reduce_browser_selection_transition(|machine| machine.publish_preview(candidate));
    }

    /// Update the controller-owned transition after browser focus is explicitly committed.
    pub(crate) fn publish_browser_commit_selection(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        entry_index: usize,
        record_focus_history: bool,
        refresh_similarity_highlight: bool,
        load_requested: bool,
        pending_playback: Option<PendingPlayback>,
    ) {
        let candidate = BrowserSelectionCandidate::new(source_id, relative_path, entry_index);
        let request = BrowserSelectionCommitRequest {
            record_focus_history,
            refresh_similarity_highlight,
            load_requested,
        };
        self.reduce_browser_selection_transition(|machine| {
            machine.publish_commit(candidate, request, pending_playback)
        });
    }

    /// Mark one browser-selection candidate as actively loading or reusing a staged load.
    pub(crate) fn note_browser_selection_loading(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        pending_playback: Option<PendingPlayback>,
    ) {
        let Some(entry_index) = self.browser_transition_entry_index(&relative_path) else {
            return;
        };
        let fallback_stage = if self.ui.browser.selection.commit_focus_pending {
            BrowserSelectionCommitStage::Preview
        } else {
            BrowserSelectionCommitStage::Settled
        };
        let candidate = BrowserSelectionCandidate::new(source_id, relative_path, entry_index);
        self.reduce_browser_selection_transition(|machine| {
            machine.note_loading(candidate, fallback_stage, pending_playback)
        });
    }

    /// Mark one browser-selection candidate as waiting on waveform visuals before handoff.
    pub(crate) fn note_browser_selection_staged(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
    ) {
        self.reduce_browser_selection_transition(|machine| {
            machine.note_awaiting_waveform(source_id, relative_path)
        });
    }

    /// Clear the transition when the matching candidate finished handoff or failed permanently.
    pub(crate) fn clear_browser_selection_transition(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
    ) {
        self.reduce_browser_selection_transition(|machine| machine.clear(source_id, relative_path));
    }

    /// Flush one deferred browser-focus commit when the browser candidate is still current.
    pub(crate) fn flush_pending_browser_focus_commit(&mut self) {
        let candidate_is_current = self
            .runtime
            .browser
            .selection_transition
            .as_ref()
            .is_some_and(|transition| self.browser_selection_transition_is_current(transition));
        self.reduce_browser_selection_transition(|machine| {
            machine.flush_commit(candidate_is_current)
        });
    }

    /// Commit only the history part of one deferred browser-selection commit, then drop it.
    pub(crate) fn abandon_pending_browser_focus_commit_for_navigation(&mut self) {
        self.reduce_browser_selection_transition(|machine| machine.abandon_for_navigation());
    }

    fn browser_transition_entry_index(&mut self, relative_path: &Path) -> Option<usize> {
        self.ui
            .browser
            .selection
            .last_focused_path
            .as_deref()
            .filter(|path| *path == relative_path)
            .and(self.ui.browser.selection.last_focused_index)
            .or_else(|| self.wav_index_for_path(relative_path))
    }

    fn browser_selection_transition_is_current(
        &self,
        transition: &BrowserSelectionTransition,
    ) -> bool {
        self.selection_state.ctx.selected_source.as_ref() == Some(&transition.source_id)
            && self.sample_view.wav.selected_wav.as_deref()
                == Some(transition.relative_path.as_path())
            && self.ui.browser.selection.last_focused_index == Some(transition.entry_index)
            && self.ui.browser.selection.last_focused_path.as_deref()
                == Some(transition.relative_path.as_path())
    }

    fn cancel_transition_load_if_needed(&mut self, transition: &BrowserSelectionTransition) {
        if !transition.load_requested() {
            return;
        }
        if self
            .runtime
            .jobs
            .pending_playback()
            .as_ref()
            .is_some_and(|playback| {
                playback.source_id == transition.source_id
                    && playback.relative_path == transition.relative_path
            })
        {
            self.runtime.jobs.set_pending_playback(None);
        }
        if self.ui.waveform.loading.as_deref() == Some(transition.relative_path.as_path()) {
            self.ui.waveform.loading = None;
        }
    }

    fn refresh_transition_similarity_highlight(&mut self, transition: &BrowserSelectionTransition) {
        if let Some(source) = self.current_source() {
            let sample_id =
                analysis_jobs::build_sample_id(source.id.as_str(), &transition.relative_path);
            self.defer_focused_similarity_highlight_refresh(
                sample_id,
                transition.relative_path.clone(),
                Some(transition.entry_index),
            );
        } else {
            self.clear_focused_similarity_highlight();
        }
    }

    fn sync_browser_commit_focus_pending(&mut self) {
        self.ui.browser.selection.commit_focus_pending = self
            .runtime
            .browser
            .selection_transition
            .as_ref()
            .is_some_and(BrowserSelectionTransition::preview_pending);
    }

    fn reduce_browser_selection_transition(
        &mut self,
        reduce: impl FnOnce(BrowserSelectionTransitionMachine) -> BrowserSelectionTransitionOutcome,
    ) {
        let current = self.runtime.browser.selection_transition.take();
        let outcome = reduce(BrowserSelectionTransitionMachine::new(current));
        self.apply_browser_selection_transition_outcome(outcome);
    }

    fn apply_browser_selection_transition_outcome(
        &mut self,
        outcome: BrowserSelectionTransitionOutcome,
    ) {
        self.runtime.browser.selection_transition = outcome.transition;
        self.sync_browser_commit_focus_pending();
        for effect in outcome.effects {
            match effect {
                BrowserSelectionTransitionEffect::RecordFocusHistory {
                    source_id,
                    relative_path,
                } => {
                    self.record_focus_history_for_source(source_id, &relative_path);
                }
                BrowserSelectionTransitionEffect::RefreshSimilarityHighlight(transition) => {
                    self.refresh_transition_similarity_highlight(&transition);
                }
                BrowserSelectionTransitionEffect::CancelLoad(transition) => {
                    self.cancel_transition_load_if_needed(&transition);
                }
                BrowserSelectionTransitionEffect::QueueAudioLoad(transition) => {
                    self.apply_browser_selection_audio_load_effect(transition);
                }
            }
        }
    }

    fn apply_browser_selection_audio_load_effect(
        &mut self,
        transition: BrowserSelectionTransition,
    ) {
        let Some(source) = self.current_source() else {
            self.runtime.jobs.set_pending_playback(None);
            self.ui.waveform.loading = None;
            self.clear_browser_selection_transition(
                &transition.source_id,
                &transition.relative_path,
            );
            return;
        };
        if let Err(err) = self.queue_browser_selection_audio_load_for(
            &source,
            &transition.relative_path,
            transition.pending_playback.clone(),
        ) {
            self.runtime.jobs.set_pending_playback(None);
            self.ui.waveform.loading = None;
            self.clear_browser_selection_transition(
                &transition.source_id,
                &transition.relative_path,
            );
            self.set_status(err, StatusTone::Error);
        }
    }
}
