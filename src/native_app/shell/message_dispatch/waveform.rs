use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{
    GuiMessage, NativeAppState, WaveformActiveDragKind, WaveformInteraction,
    WaveformPlaySelectionSnapshot, WaveformSelectionKind, emit_gui_action,
};

impl NativeAppState {
    pub(super) fn apply_waveform_message(
        &mut self,
        message: WaveformInteraction,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if let WaveformInteraction::DragLoadedSample(drag) = message {
            self.drag_loaded_waveform_sample(drag, context);
            return;
        }
        let started_at = Instant::now();
        let action = waveform_interaction_action(&message);
        let active_drag = self.waveform.current.active_drag_kind();
        let play_selection_before = self.play_selection_transaction_begin_snapshot(&message);
        if let WaveformInteraction::DragPlaySelectionExport(drag) = message
            && !self.drag_waveform_play_selection(drag, context)
        {
            return;
        }
        self.waveform.current.apply_interaction(message);
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
    play_selection_drag_active(active_drag)
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
