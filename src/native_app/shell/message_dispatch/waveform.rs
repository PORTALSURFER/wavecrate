use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{
    GuiMessage, NativeAppState, WaveformActiveDragKind, WaveformInteraction, WaveformSelectionKind,
    emit_gui_action,
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
        if let WaveformInteraction::DragPlaySelectionExport(drag) = message
            && !self.drag_waveform_play_selection(drag, context)
        {
            return;
        }
        self.waveform.current.apply_interaction(message);
        self.sync_edit_fade_audio_state();
        if waveform_interaction_finishes_play_selection_edit(&message, active_drag) {
            self.retarget_loop_playback_to_play_selection();
        }
        if let Some(action) = action {
            emit_gui_action(action, Some("waveform"), None, "applied", started_at, None);
        }
        if let Some(start_ratio) = self.waveform.current.take_pending_playback_start() {
            self.maybe_open_audio_player(context);
            self.play_waveform_from_ratio(start_ratio, context);
        }
    }
}

fn waveform_interaction_finishes_play_selection_edit(
    interaction: &WaveformInteraction,
    active_drag: Option<WaveformActiveDragKind>,
) -> bool {
    if !matches!(interaction, WaveformInteraction::FinishSelection { .. }) {
        return false;
    }
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
