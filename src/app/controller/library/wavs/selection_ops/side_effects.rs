/// Side-effect policy for wav-path selection/focus transitions.
#[derive(Clone, Copy)]
pub(super) struct SelectionSideEffects {
    /// Commit pending playback-age metadata before changing focus.
    pub(super) commit_pending_age_update: bool,
    /// Record focus history entries for random-navigation workflows.
    pub(super) record_focus_history: bool,
    /// Recompute near-duplicate similarity highlights for the focused sample.
    pub(super) refresh_similarity_highlight: bool,
    /// Queue background audio/waveform loading for the focused path.
    pub(super) queue_audio_load: bool,
}

impl SelectionSideEffects {
    /// Side effects for lightweight preview navigation.
    pub(super) const fn preview() -> Self {
        Self {
            commit_pending_age_update: false,
            record_focus_history: false,
            refresh_similarity_highlight: false,
            queue_audio_load: false,
        }
    }

    /// Side effects for full commit selection.
    pub(super) const fn commit() -> Self {
        Self {
            commit_pending_age_update: true,
            record_focus_history: true,
            refresh_similarity_highlight: true,
            queue_audio_load: true,
        }
    }
}
