//! Waveform gesture routing from pointer state into `UiAction`s.

use super::*;

mod clear;
mod drag;
mod press;
mod slices;

pub(crate) use self::slices::duplicate_cleanup_exemption_action_from_pointer;
use self::{
    clear::{waveform_clear_action_from_pointer, waveform_new_selection_action_from_pointer},
    press::{
        waveform_circular_slide_action_from_pointer,
        waveform_edit_selection_edge_adjust_action_from_pointer,
        waveform_edit_selection_shift_action_from_pointer,
        waveform_edit_selection_slide_action_from_pointer,
        waveform_primary_press_action_from_pointer, waveform_selection_drag_action_from_pointer,
        waveform_selection_edge_adjust_action_from_pointer,
        waveform_selection_resize_action_from_pointer,
        waveform_selection_shift_action_from_pointer, waveform_selection_slide_action_from_pointer,
    },
    slices::waveform_slice_toggle_action_from_pointer,
};

/// Build one waveform action from pointer position and active modifier keys.
pub(super) fn waveform_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    // Preserve the initial click/press anchor at nano precision for selection
    // creation and extension so zoomed-in gestures start exactly under the pointer.
    let pointer_position = waveform_pointer_position_from_point(layout, model, point);
    let position_nanos = pointer_position.position_nanos;
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_slice_toggle_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if command
        && alt
        && !shift
        && let Some(action) = waveform_circular_slide_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if command
        && !alt
        && let Some(action) =
            waveform_selection_edge_adjust_action_from_pointer(layout, model, point, shift)
    {
        return action;
    }
    if let Some(action) =
        waveform_primary_press_action_from_pointer(layout, model, point, command, alt, shift)
    {
        return action;
    }
    if !command
        && !alt
        && shift
        && let Some(action) = waveform_selection_slide_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if let Some(action) =
        waveform_new_selection_action_from_pointer(layout, model, point, command, alt, shift)
    {
        return action;
    }
    if let Some(action) =
        waveform_clear_action_from_pointer(layout, model, point, command, alt, shift)
    {
        return action;
    }
    if command {
        UiAction::SetWaveformCursorPrecise { position_nanos }
    } else if shift {
        UiAction::SetWaveformSelectionRangePrecise {
            start_nanos: waveform_anchor_micros(model).saturating_mul(1000),
            end_nanos: position_nanos,
            snap_override: false,
            preserve_view_edge: false,
        }
    } else {
        UiAction::BeginWaveformSelectionAtPrecise {
            anchor_nanos: position_nanos,
        }
    }
}

/// Return whether the pointer is hovering any waveform resize/fade handle.
pub(super) fn waveform_resize_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_selection_drag_action_from_pointer(layout, model, point).is_some()
        || waveform_selection_shift_action_from_pointer(layout, model, point).is_some()
        || waveform_edit_selection_shift_action_from_pointer(layout, model, point).is_some()
        || waveform_edit_resize_action_from_pointer(layout, model, point).is_some()
        || waveform_edit_fade_handle_action_from_pointer(layout, model, point).is_some()
        || waveform_selection_resize_action_from_pointer(layout, model, point, false).is_some()
        || waveform_selection_resize_action_from_pointer(layout, model, point, true).is_some()
}

/// Return whether the pointer is hovering the playback-selection drag handle.
pub(super) fn waveform_selection_drag_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_selection_drag_handle_hit_rect(layout, model).is_some_and(|rect| rect.contains(point))
}

/// Build one waveform edit-selection action from pointer position.
pub(super) fn waveform_edit_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    if !layout.waveform_plot.contains(point) {
        return UiAction::FocusWaveformPanel;
    }
    let command = modifiers.control_key() || modifiers.super_key();
    if command
        && !modifiers.alt_key()
        && let Some(action) = waveform_edit_selection_edge_adjust_action_from_pointer(
            layout,
            model,
            point,
            modifiers.shift_key(),
        )
    {
        return action;
    }
    if modifiers.alt_key()
        && let Some(action) = waveform_edit_fade_curve_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && !modifiers.alt_key()
        && modifiers.shift_key()
        && let Some(action) =
            waveform_edit_selection_slide_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if let Some(action) = waveform_edit_selection_shift_action_from_pointer(layout, model, point) {
        return action;
    }
    if let Some(action) = waveform_edit_resize_action_from_pointer(layout, model, point) {
        return action;
    }
    if let Some(action) = waveform_edit_fade_handle_action_from_pointer(layout, model, point) {
        return action;
    }
    if layout.waveform_plot.contains(point)
        && model.waveform.edit_selection_milli.is_some()
        && !waveform_edit_selection_contains_point(layout, model, point)
    {
        return UiAction::ClearWaveformEditSelection;
    }
    let position_nanos = waveform_position_nanos_from_point(layout, model, point);
    UiAction::SetWaveformEditSelectionRangePrecise {
        start_nanos: position_nanos,
        end_nanos: position_nanos,
        preserve_view_edge: false,
    }
}

#[cfg(test)]
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
    modifiers: ModifiersState,
) -> UiAction {
    drag::waveform_drag_action_for_mode(layout, model, point, mode, modifiers)
}

/// Resolve one waveform drag action and the updated drag mode for the pointer.
pub(super) fn waveform_drag_action_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
    modifiers: ModifiersState,
) -> (UiAction, WaveformPointerDragMode) {
    drag::waveform_drag_action_and_mode_for_point(layout, model, point, mode, modifiers)
}

pub(super) fn waveform_drag_exceeds_click_slop(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> bool {
    drag::waveform_drag_exceeds_click_slop(layout, model, point, mode)
}

pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    drag::waveform_drag_mode_for_action(action)
}

pub(super) fn waveform_drag_mode_is_edit_fade(mode: WaveformPointerDragMode) -> bool {
    drag::waveform_drag_mode_is_edit_fade(mode)
}

pub(super) fn waveform_press_action_emits_immediately(action: &UiAction) -> bool {
    drag::waveform_press_action_emits_immediately(action)
}
