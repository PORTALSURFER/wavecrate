use std::time::Instant;

use crate::app_core::actions::{GuiSurface, NativeUiAction, action_catalog_entry};
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use tracing::error;

use super::{
    AppController, apply_browser_native_ui_action, apply_map_native_ui_action,
    apply_prompt_and_update_native_ui_action, apply_waveform_native_ui_action,
};

/// Apply one native runtime UI action and emit action telemetry.
pub(super) fn apply_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> bool {
    let started_at = Instant::now();
    let action_id = native_action_id(&action);
    let pane = native_action_pane(&action);
    controller.begin_waveform_refresh_batch();
    let action = match apply_transport_native_ui_action(controller, action) {
        Ok(()) => {
            controller.end_waveform_refresh_batch();
            record_native_action(action_id, pane, "success", started_at, None);
            return true;
        }
        Err(action) => action,
    };
    let action = match apply_browser_native_ui_action(controller, action) {
        Ok(()) => {
            controller.end_waveform_refresh_batch();
            record_native_action(action_id, pane, "success", started_at, None);
            return true;
        }
        Err(action) => action,
    };
    let action = match apply_map_native_ui_action(controller, action) {
        Ok(()) => {
            controller.end_waveform_refresh_batch();
            record_native_action(action_id, pane, "success", started_at, None);
            return true;
        }
        Err(action) => action,
    };
    let action = match apply_waveform_native_ui_action(controller, action) {
        Ok(()) => {
            controller.end_waveform_refresh_batch();
            record_native_action(action_id, pane, "success", started_at, None);
            return true;
        }
        Err(action) => action,
    };
    let handled = match apply_prompt_and_update_native_ui_action(controller, action) {
        Ok(()) => true,
        Err(unhandled) => {
            error!(
                ?unhandled,
                "native ui action was not handled by any dispatcher group"
            );
            false
        }
    };
    controller.end_waveform_refresh_batch();
    record_native_action(
        action_id,
        pane,
        if handled { "success" } else { "unhandled" },
        started_at,
        (!handled).then_some("unhandled"),
    );
    handled
}

/// Try to dispatch transport-oriented native actions.
fn apply_transport_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SelectColumn { index } => controller.select_column_by_index(index),
        NativeUiAction::MoveColumn { delta } => controller.move_selection_column(delta as isize),
        NativeUiAction::PlayFromStart => {
            controller.play_from_start();
        }
        NativeUiAction::PlayFromCurrentPlayhead => {
            controller.play_from_current_playhead();
        }
        NativeUiAction::PlayFromWaveformCursor => {
            controller.play_from_cursor();
        }
        NativeUiAction::PlayWaveformAtPrecise { position_nanos } => {
            controller.seek_waveform_nanos(position_nanos);
        }
        NativeUiAction::ToggleTransport => controller.toggle_play_pause(),
        NativeUiAction::PlayCompareAnchor => controller.play_compare_anchor(),
        NativeUiAction::HandleEscape => controller.handle_escape(),
        NativeUiAction::ToggleLoopPlayback => controller.toggle_loop(),
        NativeUiAction::ToggleLoopLock => controller.toggle_loop_lock(),
        NativeUiAction::SetVolume { value_milli } => controller
            .set_volume_live((f32::from(value_milli.min(1000)) / 1000.0).clamp(0.0, 1.0)),
        NativeUiAction::CommitVolumeSetting => controller.commit_volume_setting(),
        NativeUiAction::Undo => controller.undo(),
        NativeUiAction::Redo => controller.redo(),
        action => return Err(action),
    }
    Ok(())
}

fn native_action_id(action: &NativeUiAction) -> &'static str {
    action_catalog_entry(action).action_id
}

fn native_action_pane(action: &NativeUiAction) -> Option<&'static str> {
    let entry = action_catalog_entry(action);
    Some(match entry.surface {
        GuiSurface::Browser => "browser",
        GuiSurface::Sources => "sources",
        GuiSurface::Waveform => "waveform",
        GuiSurface::Transport => "transport",
        GuiSurface::Map => "map",
        GuiSurface::Options => "options",
        GuiSurface::Prompt => "prompt",
        GuiSurface::Update => "update",
    })
}

fn record_native_action(
    action: &'static str,
    pane: Option<&'static str>,
    outcome: &'static str,
    started_at: Instant,
    error: Option<&'static str>,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane,
        source: None,
        outcome,
        elapsed: started_at.elapsed(),
        error,
    });
}
