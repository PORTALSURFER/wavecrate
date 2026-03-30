use super::*;
use crate::app::view_model;
use std::path::Path;

impl AppController {
    /// Store the currently focused browser sample as the compare anchor.
    ///
    /// Compare-anchor state is a transient playback aid and is intentionally
    /// excluded from snapshot-based undo/redo.
    pub(crate) fn set_compare_anchor_from_focused_browser_sample(&mut self) {
        let Some(source) = self.current_source() else {
            self.set_status(
                "Select a source before setting a compare anchor",
                StatusTone::Info,
            );
            return;
        };
        let Some(row) = self.focused_browser_row() else {
            self.set_status("Focus a sample to set a compare anchor", StatusTone::Info);
            return;
        };
        let Some(entry_index) = self.visible_browser_index(row) else {
            self.set_status("Focused sample is no longer available", StatusTone::Warning);
            return;
        };
        let Some(relative_path) = self
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            self.set_status("Focused sample is no longer available", StatusTone::Warning);
            return;
        };
        self.assign_compare_anchor(&source.id, &relative_path);
        self.set_status(
            format!(
                "Compare anchor set: {}",
                view_model::sample_display_label(&relative_path)
            ),
            StatusTone::Info,
        );
    }

    /// Replay the stored compare anchor without changing browser focus.
    pub(crate) fn play_compare_anchor(&mut self) {
        let Some(anchor) = self.sample_view.wav.compare_anchor.clone() else {
            self.set_status("Set a compare anchor first", StatusTone::Info);
            return;
        };
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == anchor.source_id)
            .cloned()
        else {
            self.clear_compare_anchor();
            self.set_status(
                "Compare anchor source is no longer available",
                StatusTone::Warning,
            );
            return;
        };
        if self
            .current_file_metadata(&source, &anchor.relative_path)
            .is_err()
        {
            self.clear_compare_anchor();
            self.set_status(
                "Compare anchor file is no longer available",
                StatusTone::Warning,
            );
            return;
        }
        let loaded_matches = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == anchor.source_id && audio.relative_path == anchor.relative_path
            });
        if loaded_matches {
            if let Err(err) = play_loaded_audio_for_path(
                self,
                &anchor.source_id,
                &anchor.relative_path,
                self.ui.waveform.loop_enabled,
                None,
            ) {
                self.set_status(err, StatusTone::Error);
            }
            return;
        }
        if self
            .runtime
            .jobs
            .pending_audio()
            .as_ref()
            .is_some_and(|pending| {
                pending.source_id == anchor.source_id
                    && pending.relative_path == anchor.relative_path
            })
        {
            self.runtime
                .jobs
                .set_pending_playback(Some(PendingPlayback {
                    source_id: anchor.source_id,
                    relative_path: anchor.relative_path,
                    looped: self.ui.waveform.loop_enabled,
                    start_override: None,
                    force_loaded_audio: true,
                }));
            return;
        }
        if let Err(err) = self.queue_audio_load_for(
            &source,
            &anchor.relative_path,
            AudioLoadIntent::Selection,
            Some(PendingPlayback {
                source_id: anchor.source_id,
                relative_path: anchor.relative_path.clone(),
                looped: self.ui.waveform.loop_enabled,
                start_override: None,
                force_loaded_audio: true,
            }),
        ) {
            self.set_status(err, StatusTone::Error);
        }
    }

    /// Clear the compare anchor state and its projected UI metadata.
    ///
    /// The compare anchor is not part of the meaningful undo/redo snapshot.
    pub(crate) fn clear_compare_anchor(&mut self) {
        self.sample_view.wav.compare_anchor = None;
        self.ui.compare_anchor = None;
        self.ui.waveform.compare_anchor_label = None;
    }

    /// Remove the compare anchor when it references the provided sample path.
    pub(crate) fn clear_compare_anchor_if_matches(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
    ) {
        if self
            .sample_view
            .wav
            .compare_anchor
            .as_ref()
            .is_some_and(|anchor| {
                &anchor.source_id == source_id && anchor.relative_path.as_path() == relative_path
            })
        {
            self.clear_compare_anchor();
        }
    }

    /// Rewrite the compare-anchor path after an in-place rename or move.
    pub(crate) fn update_compare_anchor_path(
        &mut self,
        source_id: &SourceId,
        old_path: &Path,
        new_path: &Path,
    ) {
        let Some(anchor) = self.sample_view.wav.compare_anchor.as_mut() else {
            return;
        };
        if &anchor.source_id != source_id || anchor.relative_path.as_path() != old_path {
            return;
        }
        anchor.relative_path = new_path.to_path_buf();
        let label = view_model::sample_display_label(new_path);
        self.ui.waveform.compare_anchor_label = Some(label.clone());
        self.ui.compare_anchor = Some(crate::app::state::CompareAnchorState {
            source_id: source_id.clone(),
            relative_path: new_path.to_path_buf(),
            label,
        });
    }

    fn assign_compare_anchor(&mut self, source_id: &SourceId, relative_path: &Path) {
        let label = view_model::sample_display_label(relative_path);
        self.sample_view.wav.compare_anchor = Some(CompareAnchorSample {
            source_id: source_id.clone(),
            relative_path: relative_path.to_path_buf(),
        });
        self.ui.compare_anchor = Some(crate::app::state::CompareAnchorState {
            source_id: source_id.clone(),
            relative_path: relative_path.to_path_buf(),
            label: label.clone(),
        });
        self.ui.waveform.compare_anchor_label = Some(label);
    }
}

/// Play the currently loaded audio for one explicit sample identity.
///
/// Compare-anchor replay can intentionally diverge from browser focus. This
/// helper preserves the user's visible browser focus while temporarily routing
/// transport playback through the loaded sample identity.
pub(crate) fn play_loaded_audio_for_path(
    controller: &mut AppController,
    source_id: &SourceId,
    relative_path: &Path,
    looped: bool,
    start_override: Option<f64>,
) -> Result<(), String> {
    let original_source = controller.selection_state.ctx.selected_source.clone();
    let original_selected = controller.sample_view.wav.selected_wav.clone();
    controller.selection_state.ctx.selected_source = Some(source_id.clone());
    controller.sample_view.wav.selected_wav = Some(relative_path.to_path_buf());
    let result = controller.play_audio(looped, start_override);
    controller.selection_state.ctx.selected_source = original_source;
    controller.sample_view.wav.selected_wav = original_selected;
    result
}
