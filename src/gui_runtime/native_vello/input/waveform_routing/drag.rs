use super::*;

#[cfg(test)]
/// Resolve one waveform action for a captured waveform drag mode.
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
    modifiers: ModifiersState,
) -> UiAction {
    waveform_drag_action_and_mode_for_point(layout, model, point, mode, modifiers).0
}

/// Resolve one waveform action and updated drag mode for a captured waveform drag.
pub(super) fn waveform_drag_action_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
    modifiers: ModifiersState,
) -> (UiAction, WaveformPointerDragMode) {
    let pointer_position = waveform_pointer_position_from_point(layout, model, point);
    let position_nanos = pointer_position.position_nanos;
    let (position_nanos_for_selection, next_mode) =
        waveform_drag_position_and_mode_for_point(layout, model, point, mode);
    let position_micros = nanos_to_micros(position_nanos_for_selection);
    let preserve_view_edge = waveform_point_is_outside_plot_x(layout, point);
    let action = match next_mode {
        WaveformPointerDragMode::Seek => UiAction::SeekWaveformPrecise { position_nanos },
        WaveformPointerDragMode::Cursor => UiAction::SetWaveformCursorPrecise { position_nanos },
        WaveformPointerDragMode::CircularSlide { .. } => {
            UiAction::UpdateWaveformCircularSlide { position_micros }
        }
        WaveformPointerDragMode::Selection { anchor_micros, .. } => {
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos: anchor_micros,
                end_nanos: position_nanos_for_selection,
                snap_override: modifiers.alt_key(),
                preserve_view_edge,
            }
        }
        WaveformPointerDragMode::SelectionSmartScale { anchor_micros, .. } => {
            UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos: anchor_micros,
                end_nanos: position_nanos_for_selection,
            }
        }
        WaveformPointerDragMode::SelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => {
            let (start_nanos, end_nanos) = shift_waveform_range_nanos(
                pointer_micros,
                position_nanos_for_selection,
                start_micros,
                end_micros,
            );
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override: modifiers.alt_key(),
                preserve_view_edge: false,
            }
        }
        WaveformPointerDragMode::EditSelection { anchor_micros, .. } => {
            UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos: anchor_micros,
                end_nanos: position_nanos_for_selection,
                preserve_view_edge,
            }
        }
        WaveformPointerDragMode::EditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => {
            let (start_nanos, end_nanos) = shift_waveform_range_nanos(
                pointer_micros,
                position_nanos_for_selection,
                start_micros,
                end_micros,
            );
            UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge: false,
            }
        }
        WaveformPointerDragMode::EditFadeInEnd => {
            UiAction::SetWaveformEditFadeInEnd { position_micros }
        }
        WaveformPointerDragMode::EditFadeInMuteStart => {
            UiAction::SetWaveformEditFadeInMuteStart { position_micros }
        }
        WaveformPointerDragMode::EditFadeInCurve => UiAction::SetWaveformEditFadeInCurve {
            curve_milli: waveform_edit_fade_curve_milli_from_point(layout, point),
        },
        WaveformPointerDragMode::EditFadeOutStart => {
            UiAction::SetWaveformEditFadeOutStart { position_micros }
        }
        WaveformPointerDragMode::EditFadeOutMuteEnd => {
            UiAction::SetWaveformEditFadeOutMuteEnd { position_micros }
        }
        WaveformPointerDragMode::EditFadeOutCurve => UiAction::SetWaveformEditFadeOutCurve {
            curve_milli: waveform_edit_fade_curve_milli_from_point(layout, point),
        },
    };
    (action, next_mode)
}

/// Return whether an armed waveform drag moved far enough to emit selection updates.
///
/// New playback-selection drags use a small horizontal click-slop so minor
/// pointer wobble still behaves like a click/seek instead of creating a
/// micro-selection.
pub(super) fn waveform_drag_exceeds_click_slop(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> bool {
    match mode {
        WaveformPointerDragMode::CircularSlide { .. } => true,
        WaveformPointerDragMode::Selection { anchor_micros, .. } => {
            let anchor_x =
                waveform_x_for_micros(layout.waveform_plot, model, nanos_to_micros(anchor_micros));
            (point.x - anchor_x).abs() > WAVEFORM_SELECTION_CLICK_SLOP_PX
        }
        _ => true,
    }
}

/// Resolve drag mode from an initial waveform action emitted on pointer press.
pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    match action {
        UiAction::SeekWaveformPrecise { .. } | UiAction::SeekWaveform { .. } => {
            Some(WaveformPointerDragMode::Seek)
        }
        UiAction::SetWaveformCursorPrecise { .. } | UiAction::SetWaveformCursor { .. } => {
            Some(WaveformPointerDragMode::Cursor)
        }
        UiAction::BeginWaveformCircularSlide { anchor_micros } => {
            Some(WaveformPointerDragMode::CircularSlide {
                anchor_micros: *anchor_micros,
            })
        }
        UiAction::BeginWaveformSelectionAt { anchor_micros } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: anchor_micros.saturating_mul(1000),
                boundary_lock: None,
            })
        }
        UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: *anchor_nanos,
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformSelectionRange { start_micros, .. } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: start_micros.saturating_mul(1000),
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformSelectionRangePrecise { start_nanos, .. } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: *start_nanos,
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformSelectionRangeSmartScale { start_micros, .. } => {
            Some(WaveformPointerDragMode::SelectionSmartScale {
                anchor_micros: start_micros.saturating_mul(1000),
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformSelectionRangeSmartScalePrecise { start_nanos, .. } => {
            Some(WaveformPointerDragMode::SelectionSmartScale {
                anchor_micros: *start_nanos,
                boundary_lock: None,
            })
        }
        UiAction::BeginWaveformSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Some(WaveformPointerDragMode::SelectionShift {
            pointer_micros: pointer_micros.saturating_mul(1000),
            start_micros: start_micros.saturating_mul(1000),
            end_micros: end_micros.saturating_mul(1000),
        }),
        UiAction::BeginWaveformSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        } => Some(WaveformPointerDragMode::SelectionShift {
            pointer_micros: *pointer_nanos,
            start_micros: *start_nanos,
            end_micros: *end_nanos,
        }),
        UiAction::SetWaveformEditSelectionRange { start_micros, .. } => {
            Some(WaveformPointerDragMode::EditSelection {
                anchor_micros: start_micros.saturating_mul(1000),
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformEditSelectionRangePrecise { start_nanos, .. } => {
            Some(WaveformPointerDragMode::EditSelection {
                anchor_micros: *start_nanos,
                boundary_lock: None,
            })
        }
        UiAction::BeginWaveformEditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Some(WaveformPointerDragMode::EditSelectionShift {
            pointer_micros: pointer_micros.saturating_mul(1000),
            start_micros: start_micros.saturating_mul(1000),
            end_micros: end_micros.saturating_mul(1000),
        }),
        UiAction::BeginWaveformEditSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        } => Some(WaveformPointerDragMode::EditSelectionShift {
            pointer_micros: *pointer_nanos,
            start_micros: *start_nanos,
            end_micros: *end_nanos,
        }),
        UiAction::SetWaveformEditFadeInEnd { .. } => Some(WaveformPointerDragMode::EditFadeInEnd),
        UiAction::SetWaveformEditFadeInMuteStart { .. } => {
            Some(WaveformPointerDragMode::EditFadeInMuteStart)
        }
        UiAction::SetWaveformEditFadeInCurve { .. } => {
            Some(WaveformPointerDragMode::EditFadeInCurve)
        }
        UiAction::SetWaveformEditFadeOutStart { .. } => {
            Some(WaveformPointerDragMode::EditFadeOutStart)
        }
        UiAction::SetWaveformEditFadeOutMuteEnd { .. } => {
            Some(WaveformPointerDragMode::EditFadeOutMuteEnd)
        }
        UiAction::SetWaveformEditFadeOutCurve { .. } => {
            Some(WaveformPointerDragMode::EditFadeOutCurve)
        }
        UiAction::PlayFromWaveformCursor | UiAction::PlayWaveformAtPrecise { .. } => None,
        _ => None,
    }
}

/// Return whether one waveform drag mode edits fade geometry and needs a release callback.
pub(super) fn waveform_drag_mode_is_edit_fade(mode: WaveformPointerDragMode) -> bool {
    matches!(
        mode,
        WaveformPointerDragMode::EditFadeInEnd
            | WaveformPointerDragMode::EditFadeInMuteStart
            | WaveformPointerDragMode::EditFadeInCurve
            | WaveformPointerDragMode::EditFadeOutStart
            | WaveformPointerDragMode::EditFadeOutMuteEnd
            | WaveformPointerDragMode::EditFadeOutCurve
    )
}

/// Return whether one waveform press action should mutate model state immediately.
///
/// Selection/edit/fade gestures are armed on press and only emit once the
/// pointer actually moves. This keeps simple clicks from creating incidental
/// selection artifacts or nudging handles without a drag.
pub(super) fn waveform_press_action_emits_immediately(action: &UiAction) -> bool {
    !matches!(
        action,
        UiAction::SetWaveformSelectionRange { .. }
            | UiAction::SetWaveformSelectionRangePrecise { .. }
            | UiAction::SetWaveformSelectionRangeSmartScale { .. }
            | UiAction::SetWaveformSelectionRangeSmartScalePrecise { .. }
            | UiAction::BeginWaveformSelectionShift { .. }
            | UiAction::BeginWaveformSelectionShiftPrecise { .. }
            | UiAction::BeginWaveformSelectionAt { .. }
            | UiAction::BeginWaveformSelectionAtPrecise { .. }
            | UiAction::SetWaveformEditSelectionRange { .. }
            | UiAction::SetWaveformEditSelectionRangePrecise { .. }
            | UiAction::BeginWaveformEditSelectionShift { .. }
            | UiAction::BeginWaveformEditSelectionShiftPrecise { .. }
            | UiAction::SetWaveformEditFadeInEnd { .. }
            | UiAction::SetWaveformEditFadeInMuteStart { .. }
            | UiAction::SetWaveformEditFadeInCurve { .. }
            | UiAction::SetWaveformEditFadeOutStart { .. }
            | UiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | UiAction::SetWaveformEditFadeOutCurve { .. }
    )
}

/// Resolve one absolute waveform position and next drag-mode lock state for the pointer.
fn waveform_drag_position_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> (u32, WaveformPointerDragMode) {
    match mode {
        WaveformPointerDragMode::Selection {
            anchor_micros,
            boundary_lock,
        } => {
            let (position_nanos, boundary_lock) =
                waveform_selection_boundary_lock_for_point(layout, model, point, boundary_lock);
            (
                position_nanos,
                WaveformPointerDragMode::Selection {
                    anchor_micros,
                    boundary_lock,
                },
            )
        }
        WaveformPointerDragMode::SelectionSmartScale {
            anchor_micros,
            boundary_lock,
        } => {
            let (position_nanos, boundary_lock) =
                waveform_selection_boundary_lock_for_point(layout, model, point, boundary_lock);
            (
                position_nanos,
                WaveformPointerDragMode::SelectionSmartScale {
                    anchor_micros,
                    boundary_lock,
                },
            )
        }
        WaveformPointerDragMode::EditSelection {
            anchor_micros,
            boundary_lock,
        } => {
            let (position_nanos, boundary_lock) =
                waveform_selection_boundary_lock_for_point(layout, model, point, boundary_lock);
            (
                position_nanos,
                WaveformPointerDragMode::EditSelection {
                    anchor_micros,
                    boundary_lock,
                },
            )
        }
        _ => (
            waveform_position_nanos_from_point(layout, model, point),
            mode,
        ),
    }
}

/// Keep anchor-based drags pinned to one absolute edge while the pointer remains off-plot.
fn waveform_selection_boundary_lock_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    boundary_lock: Option<WaveformSelectionBoundaryLock>,
) -> (u32, Option<WaveformSelectionBoundaryLock>) {
    let Some(side) = waveform_outside_plot_side(layout, point) else {
        return (
            waveform_position_nanos_from_point(layout, model, point),
            None,
        );
    };
    if let Some(boundary_lock) = boundary_lock.filter(|lock| lock.side == side) {
        return (boundary_lock.position_nanos, Some(boundary_lock));
    }
    let position_nanos = waveform_position_nanos_from_point(layout, model, point);
    (
        position_nanos,
        Some(WaveformSelectionBoundaryLock {
            side,
            position_nanos,
        }),
    )
}

/// Return which horizontal waveform-plot side the pointer is currently beyond.
fn waveform_outside_plot_side(
    layout: &ShellLayout,
    point: Point,
) -> Option<WaveformOutsidePlotSide> {
    if point.x < layout.waveform_plot.min.x {
        Some(WaveformOutsidePlotSide::Left)
    } else if point.x > layout.waveform_plot.max.x {
        Some(WaveformOutsidePlotSide::Right)
    } else {
        None
    }
}
