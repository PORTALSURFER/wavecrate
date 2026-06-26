use radiant::prelude as ui;
use std::{path::PathBuf, time::Instant};
use wavecrate::selection::SelectionRange;

use crate::native_app::app::{
    ClipboardHandoffTarget, GuiMessage, NativeAppState, WaveformActiveDragKind,
    WaveformContextMenu, WaveformInteraction, WaveformPlaySelectionSnapshot, WaveformSelectionKind,
    emit_gui_action,
};

impl NativeAppState {
    pub(super) fn apply_waveform_message(
        &mut self,
        message: WaveformInteraction,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if let WaveformInteraction::DragLoadedSample(drag) = message {
            self.ui.browser_interaction.clipboard_handoff_target =
                ClipboardHandoffTarget::BrowserFiles;
            self.drag_loaded_waveform_sample(drag, context);
            return;
        }
        if let WaveformInteraction::OpenPlaySelectionContextMenu { position } = message {
            self.ui.browser_interaction.clipboard_handoff_target =
                ClipboardHandoffTarget::WaveformSelection;
            self.open_play_selection_context_menu(position);
            return;
        }
        self.ui.browser_interaction.clipboard_handoff_target =
            ClipboardHandoffTarget::WaveformSelection;
        let started_at = Instant::now();
        let action = waveform_interaction_action(&message);
        let active_drag = self.waveform.current.active_drag_kind();
        let play_selection_before = self.play_selection_transaction_begin_snapshot(&message);
        let harvest_mark_before = waveform_interaction_can_finish_mark_change(&message)
            .then(|| WaveformHarvestMarkSnapshot::from_state(self))
            .flatten();
        if let WaveformInteraction::DragPlaySelectionExport(drag) = message
            && !self.drag_waveform_play_selection(drag, context)
        {
            return;
        }
        self.waveform.current.apply_interaction(message);
        self.mark_harvest_touched_after_waveform_mark_change(harvest_mark_before);
        if let Some(before) = play_selection_before {
            self.waveform.pending_play_selection_transaction =
                play_selection_drag_active(self.waveform.current.active_drag_kind())
                    .then_some(before);
        }
        self.sync_edit_fade_audio_state();
        if waveform_interaction_updates_play_selection(&message, active_drag) {
            self.retarget_playback_to_play_selection();
        }
        if play_selection_transaction_finishes(&message, active_drag) {
            self.register_finished_play_selection_transaction();
        }
        if let Some(action) = action {
            emit_gui_action(action, Some("waveform"), None, "applied", started_at, None);
        }
        if let Some(start_ratio) = self.waveform.current.take_pending_playback_start() {
            self.maybe_open_audio_player(context);
            self.play_waveform_from_ratio(start_ratio, context);
        }
    }

    fn play_selection_transaction_begin_snapshot(
        &self,
        interaction: &WaveformInteraction,
    ) -> Option<WaveformPlaySelectionSnapshot> {
        let begins_play_selection_change = matches!(
            interaction,
            WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                ..
            } | WaveformInteraction::BeginSelectionResize {
                kind: WaveformSelectionKind::Play,
                ..
            } | WaveformInteraction::BeginSelectionMove {
                kind: WaveformSelectionKind::Play,
                ..
            }
        );
        begins_play_selection_change
            .then(|| WaveformPlaySelectionSnapshot::from_waveform(&self.waveform.current))
    }

    fn register_finished_play_selection_transaction(&mut self) {
        let Some(before) = self.waveform.pending_play_selection_transaction.take() else {
            return;
        };
        let after = WaveformPlaySelectionSnapshot::from_waveform(&self.waveform.current);
        if before.path != after.path || before == after {
            return;
        }
        let undo_snapshot = before.clone();
        let redo_snapshot = after;
        self.register_transaction_action(
            "Change play mark selection",
            move |transaction| transaction.restore_play_selection(undo_snapshot.clone()),
            move |transaction| transaction.restore_play_selection(redo_snapshot.clone()),
        );
    }

    fn open_play_selection_context_menu(&mut self, position: ui::Point) {
        if !self
            .waveform
            .current
            .play_selection()
            .is_some_and(|selection| selection.width() > 0.0)
        {
            return;
        }
        self.ui.browser_interaction.context_menu = None;
        let loaded_path = self.waveform.current.path();
        self.ui.browser_interaction.waveform_context_menu = Some(WaveformContextMenu {
            anchor: position,
            title: String::from("Playmark Selection"),
            extract_to_harvest_destination: self
                .playmark_harvest_destination_action_available(&loaded_path),
        });
        emit_gui_action(
            "waveform.playmark_context_menu.open",
            Some("waveform"),
            None,
            "opened",
            Instant::now(),
            None,
        );
    }

    fn playmark_harvest_destination_action_available(&self, path: &std::path::Path) -> bool {
        let source_known = self
            .library
            .folder_browser
            .sample_source_for_file_path(path)
            .is_some();
        source_known
            && (self.library.folder_browser.harvest_filter().is_some()
                || self
                    .library
                    .folder_browser
                    .path_is_in_protected_source(path))
    }

    fn mark_harvest_touched_after_waveform_mark_change(
        &self,
        before: Option<WaveformHarvestMarkSnapshot>,
    ) {
        let Some(before) = before else {
            return;
        };
        let Some(after) = WaveformHarvestMarkSnapshot::from_state(self) else {
            return;
        };
        if before.path == after.path && before.marks_changed(&after) {
            self.mark_harvest_touched_for_path(&after.path);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct WaveformHarvestMarkSnapshot {
    path: PathBuf,
    play_selection: Option<SelectionRange>,
    edit_selection: Option<SelectionRange>,
}

impl WaveformHarvestMarkSnapshot {
    fn from_state(state: &NativeAppState) -> Option<Self> {
        state.waveform.current.has_loaded_sample().then(|| Self {
            path: state.waveform.current.path(),
            play_selection: state.waveform.current.play_selection(),
            edit_selection: state.waveform.current.edit_selection(),
        })
    }

    fn marks_changed(&self, other: &Self) -> bool {
        self.play_selection != other.play_selection || self.edit_selection != other.edit_selection
    }
}

fn waveform_interaction_updates_play_selection(
    interaction: &WaveformInteraction,
    active_drag: Option<WaveformActiveDragKind>,
) -> bool {
    if !matches!(
        interaction,
        WaveformInteraction::UpdateSelection { .. } | WaveformInteraction::FinishSelection { .. }
    ) {
        return false;
    }
    match active_drag {
        Some(WaveformActiveDragKind::Selection(WaveformSelectionKind::Play)) => {
            matches!(interaction, WaveformInteraction::FinishSelection { .. })
        }
        Some(
            WaveformActiveDragKind::SelectionResize(WaveformSelectionKind::Play, _)
            | WaveformActiveDragKind::SelectionMove(WaveformSelectionKind::Play),
        ) => true,
        _ => false,
    }
}

fn waveform_interaction_can_finish_mark_change(interaction: &WaveformInteraction) -> bool {
    matches!(
        interaction,
        WaveformInteraction::FinishSelection { .. }
            | WaveformInteraction::SelectSimilarSection { .. }
            | WaveformInteraction::ClearEditFadeSilence { .. }
            | WaveformInteraction::FinishEditFadeOuterGain { .. }
            | WaveformInteraction::FinishEditGain { .. }
    )
}

fn play_selection_transaction_finishes(
    interaction: &WaveformInteraction,
    active_drag: Option<WaveformActiveDragKind>,
) -> bool {
    matches!(interaction, WaveformInteraction::FinishSelection { .. })
        && play_selection_drag_active(active_drag)
}

fn play_selection_drag_active(active_drag: Option<WaveformActiveDragKind>) -> bool {
    matches!(
        active_drag,
        Some(WaveformActiveDragKind::Selection(
            WaveformSelectionKind::Play
        )) | Some(WaveformActiveDragKind::SelectionResize(
            WaveformSelectionKind::Play,
            _
        )) | Some(WaveformActiveDragKind::SelectionMove(
            WaveformSelectionKind::Play
        ))
    )
}

fn waveform_interaction_action(interaction: &WaveformInteraction) -> Option<&'static str> {
    match interaction {
        WaveformInteraction::Wheel { .. } => Some("waveform.zoom_wheel"),
        WaveformInteraction::ZoomToPlaySelection => Some("waveform.zoom_to_play_selection"),
        WaveformInteraction::ZoomFull => Some("waveform.zoom_full"),
        WaveformInteraction::ZoomOut {
            expand_silence_margin: true,
        } => Some("waveform.zoom_out_silence_margin"),
        WaveformInteraction::ZoomOut {
            expand_silence_margin: false,
        } => Some("waveform.zoom_out"),
        WaveformInteraction::OpenPlaySelectionContextMenu { .. } => {
            Some("waveform.playmark_context_menu.open")
        }
        WaveformInteraction::ScrollTo { .. } => Some("waveform.scroll"),
        WaveformInteraction::BeginSelection { .. } => Some("waveform.selection.begin"),
        WaveformInteraction::BeginEditFade { .. } => Some("waveform.edit_fade.begin"),
        WaveformInteraction::BeginEditFadeOuterGain { .. } => {
            Some("waveform.edit_fade_outer_gain.begin")
        }
        WaveformInteraction::FinishEditFadeOuterGain { .. } => {
            Some("waveform.edit_fade_outer_gain.finish")
        }
        WaveformInteraction::BeginEditGain { .. } => Some("waveform.edit_gain.begin"),
        WaveformInteraction::FinishEditGain { .. } => Some("waveform.edit_gain.finish"),
        WaveformInteraction::ClearEditFadeSilence { .. } => {
            Some("waveform.edit_fade.clear_silence")
        }
        WaveformInteraction::SelectSimilarSection { .. } => Some("waveform.similar_section.select"),
        WaveformInteraction::BeginSelectionResize { .. } => Some("waveform.selection.resize_begin"),
        WaveformInteraction::BeginSelectionMove { .. } => Some("waveform.selection.move_begin"),
        WaveformInteraction::BeginPan { .. } => Some("waveform.pan_begin"),
        WaveformInteraction::DragPlaySelectionExport(drag) => match drag.phase() {
            ui::DragHandlePhase::Started => Some("waveform.selection_export_drag.begin"),
            ui::DragHandlePhase::Moved => None,
            ui::DragHandlePhase::Ended => Some("waveform.selection_export_drag.end"),
            ui::DragHandlePhase::DoubleActivate => None,
            ui::DragHandlePhase::Cancelled => None,
        },
        WaveformInteraction::DragLoadedSample(_) => None,
        WaveformInteraction::FinishSelection { .. } => Some("waveform.selection.finish"),
        WaveformInteraction::UpdateSelection { .. }
        | WaveformInteraction::UpdateEditFadeOuterGain { .. }
        | WaveformInteraction::UpdateEditGain { .. }
        | WaveformInteraction::Frame => None,
    }
}

#[cfg(test)]
mod tests {
    use radiant::gui::types::Point;

    use super::*;
    use crate::native_app::waveform::{WaveformEditFadeHandle, WaveformSelectionEdge};

    #[test]
    fn live_selection_updates_do_not_finish_harvest_mark_changes() {
        assert!(!waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.1,
            }
        ));
        assert!(!waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::UpdateSelection { visible_ratio: 0.4 }
        ));
        assert!(!waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::UpdateEditGain { pointer_y: 24.0 }
        ));
        assert!(!waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::UpdateEditFadeOuterGain {
                vertical_ratio: 0.4,
            }
        ));
        assert!(!waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::OpenPlaySelectionContextMenu {
                position: Point::new(10.0, 10.0),
            }
        ));
    }

    #[test]
    fn finished_and_discrete_selection_edits_finish_harvest_mark_changes() {
        assert!(waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::FinishSelection { visible_ratio: 0.4 }
        ));
        assert!(waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::SelectSimilarSection {
                selection: SelectionRange::new(0.2, 0.4),
            }
        ));
        assert!(waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::ClearEditFadeSilence {
                handle: WaveformEditFadeHandle::InOuterStart,
            }
        ));
        assert!(waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::FinishEditGain { pointer_y: 12.0 }
        ));
        assert!(waveform_interaction_can_finish_mark_change(
            &WaveformInteraction::FinishEditFadeOuterGain {
                vertical_ratio: 0.4,
            }
        ));
    }

    #[test]
    fn freshly_painted_playmark_retargets_playback_only_on_release() {
        let active_drag = Some(WaveformActiveDragKind::Selection(
            WaveformSelectionKind::Play,
        ));

        assert!(!waveform_interaction_updates_play_selection(
            &WaveformInteraction::UpdateSelection {
                visible_ratio: 0.21
            },
            active_drag,
        ));
        assert!(waveform_interaction_updates_play_selection(
            &WaveformInteraction::FinishSelection {
                visible_ratio: 0.40
            },
            active_drag,
        ));
    }

    #[test]
    fn existing_playmark_edits_keep_live_playback_retargeting() {
        for active_drag in [
            Some(WaveformActiveDragKind::SelectionResize(
                WaveformSelectionKind::Play,
                WaveformSelectionEdge::End,
            )),
            Some(WaveformActiveDragKind::SelectionMove(
                WaveformSelectionKind::Play,
            )),
        ] {
            assert!(waveform_interaction_updates_play_selection(
                &WaveformInteraction::UpdateSelection {
                    visible_ratio: 0.35
                },
                active_drag,
            ));
            assert!(waveform_interaction_updates_play_selection(
                &WaveformInteraction::FinishSelection {
                    visible_ratio: 0.45
                },
                active_drag,
            ));
        }
    }
}
