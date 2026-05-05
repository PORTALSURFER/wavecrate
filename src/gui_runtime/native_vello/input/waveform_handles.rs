//! Waveform selection/edit/fade handle hit-testing helpers.

use super::*;
use crate::gui::range::NormalizedRange;

/// Resolve one fade-handle action when a pointer lands near edit fade handles.
pub(super) fn waveform_edit_fade_handle_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    if selection_end <= selection_start {
        return None;
    }
    let fade_in_end_micros = model
        .waveform
        .edit_fade_in_end_micros
        .unwrap_or(selection_start)
        .clamp(selection_start, selection_end);
    let has_fade_in = fade_in_end_micros > selection_start;
    let fade_in_mute_start_micros = model
        .waveform
        .edit_fade_in_mute_start_micros
        .unwrap_or(selection_start)
        .min(selection_start);
    let fade_out_start_micros = model
        .waveform
        .edit_fade_out_start_micros
        .unwrap_or(selection_end)
        .clamp(selection_start, selection_end);
    let has_fade_out = fade_out_start_micros < selection_end;
    let fade_out_mute_end_micros = model
        .waveform
        .edit_fade_out_mute_end_micros
        .unwrap_or(selection_end)
        .max(selection_end);
    let selection_start_x = waveform_x_for_micros(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_micros(layout.waveform_plot, model, selection_end);
    let selection_rect = UiRect::from_min_max(
        Point::new(
            selection_start_x.min(selection_end_x),
            layout.waveform_plot.min.y,
        ),
        Point::new(
            selection_start_x.max(selection_end_x),
            layout.waveform_plot.max.y,
        ),
    );
    let fade_in_x = waveform_x_for_micros(layout.waveform_plot, model, fade_in_end_micros);
    let fade_in_mute_x =
        waveform_x_for_micros(layout.waveform_plot, model, fade_in_mute_start_micros);
    let fade_out_x = waveform_x_for_micros(layout.waveform_plot, model, fade_out_start_micros);
    let fade_out_mute_x =
        waveform_x_for_micros(layout.waveform_plot, model, fade_out_mute_end_micros);
    let in_top_hit =
        waveform_edit_fade_top_handle_hit_rect(layout.waveform_plot, selection_rect, fade_in_x)
            .contains(point);
    let out_top_hit =
        waveform_edit_fade_top_handle_hit_rect(layout.waveform_plot, selection_rect, fade_out_x)
            .contains(point);
    let threshold = WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH;
    let bottom_half = point.y >= layout.waveform_plot.center().y;
    let in_bottom_hit = has_fade_in && bottom_half && (point.x - fade_in_mute_x).abs() <= threshold;
    let out_bottom_hit =
        has_fade_out && bottom_half && (point.x - fade_out_mute_x).abs() <= threshold;
    if !in_top_hit && !out_top_hit && !in_bottom_hit && !out_bottom_hit {
        return None;
    }
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    if in_bottom_hit && (!out_bottom_hit || point.x <= (fade_in_mute_x + fade_out_mute_x) * 0.5) {
        Some(UiAction::SetWaveformEditFadeInMuteStart { position_micros })
    } else if out_bottom_hit {
        Some(UiAction::SetWaveformEditFadeOutMuteEnd { position_micros })
    } else if in_top_hit && (!out_top_hit || point.x <= (fade_in_x + fade_out_x) * 0.5) {
        Some(UiAction::SetWaveformEditFadeInEnd { position_micros })
    } else {
        Some(UiAction::SetWaveformEditFadeOutStart { position_micros })
    }
}

/// Resolve one edit-fade curve action when Alt is held over a fade region or handle.
pub(super) fn waveform_edit_fade_curve_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    if selection_end <= selection_start {
        return None;
    }
    let fade_in_end_micros = model
        .waveform
        .edit_fade_in_end_micros
        .unwrap_or(selection.start_micros)
        .clamp(selection_start, selection_end);
    let fade_in_mute_start_micros = model
        .waveform
        .edit_fade_in_mute_start_micros
        .unwrap_or(selection_start)
        .min(selection_start);
    let fade_out_start_micros = model
        .waveform
        .edit_fade_out_start_micros
        .unwrap_or(selection.end_micros)
        .clamp(selection_start, selection_end);
    let fade_out_mute_end_micros = model
        .waveform
        .edit_fade_out_mute_end_micros
        .unwrap_or(selection_end)
        .max(selection_end);
    let fade_in_mute_x =
        waveform_x_for_micros(layout.waveform_plot, model, fade_in_mute_start_micros);
    let selection_start_x = waveform_x_for_micros(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_micros(layout.waveform_plot, model, selection_end);
    let fade_in_x = waveform_x_for_micros(layout.waveform_plot, model, fade_in_end_micros);
    let fade_out_x = waveform_x_for_micros(layout.waveform_plot, model, fade_out_start_micros);
    let fade_out_mute_x =
        waveform_x_for_micros(layout.waveform_plot, model, fade_out_mute_end_micros);
    let threshold = WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH;
    let in_region_hit = point.x >= fade_in_mute_x - threshold && point.x <= fade_in_x + threshold;
    let out_region_hit =
        point.x >= fade_out_x - threshold && point.x <= fade_out_mute_x + threshold;
    let curve_milli = waveform_edit_fade_curve_milli_from_point(layout, point);
    if in_region_hit && (!out_region_hit || point.x <= (selection_start_x + selection_end_x) * 0.5)
    {
        return Some(UiAction::SetWaveformEditFadeInCurve { curve_milli });
    }
    if out_region_hit {
        return Some(UiAction::SetWaveformEditFadeOutCurve { curve_milli });
    }
    None
}

/// Resolve one edit-selection resize action when the pointer lands on an edge handle.
pub(super) fn waveform_edit_resize_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    if !layout.waveform_plot.contains(point)
        || !waveform_edit_selection_contains_point(layout, model, point)
    {
        return None;
    }
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    if selection_end <= selection_start {
        return None;
    }
    let selection_start_x = waveform_x_for_micros(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_micros(layout.waveform_plot, model, selection_end);
    let (handle_top, handle_bottom) = waveform_centered_resize_edge_y_bounds(layout.waveform_plot);
    if point.y < handle_top || point.y > handle_bottom {
        return None;
    }
    let left_distance = (point.x - selection_start_x).abs();
    let right_distance = (point.x - selection_end_x).abs();
    let left_hit = left_distance <= WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH;
    let right_hit = right_distance <= WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH;
    if !left_hit && !right_hit {
        return None;
    }
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    if left_hit && (!right_hit || left_distance <= right_distance) {
        return Some(UiAction::SetWaveformEditSelectionRange {
            start_micros: selection.end_micros,
            end_micros: position_micros,
            preserve_view_edge: false,
        });
    }
    Some(UiAction::SetWaveformEditSelectionRange {
        start_micros: selection.start_micros,
        end_micros: position_micros,
        preserve_view_edge: false,
    })
}

/// Return whether a waveform point lands inside the current edit-selection body.
pub(super) fn waveform_edit_selection_contains_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    let selection = match model.waveform.edit_selection_milli {
        Some(selection) if layout.waveform_plot.contains(point) => selection,
        _ => return false,
    };
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    if selection_end <= selection_start {
        return false;
    }
    let start_x = waveform_x_for_micros(layout.waveform_plot, model, selection_start);
    let end_x = waveform_x_for_micros(layout.waveform_plot, model, selection_end);
    UiRect::from_min_max(
        Point::new(start_x.min(end_x), layout.waveform_plot.min.y),
        Point::new(start_x.max(end_x), layout.waveform_plot.max.y),
    )
    .contains(point)
}

/// Return whether a waveform point lands inside the current playback-selection body.
pub(super) fn waveform_selection_contains_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    let selection = match model.waveform.selection_milli {
        Some(selection) if layout.waveform_plot.contains(point) => selection,
        _ => return false,
    };
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    if selection_end <= selection_start {
        return false;
    }
    let start_x = waveform_x_for_micros(layout.waveform_plot, model, selection_start);
    let end_x = waveform_x_for_micros(layout.waveform_plot, model, selection_end);
    UiRect::from_min_max(
        Point::new(start_x.min(end_x), layout.waveform_plot.min.y),
        Point::new(start_x.max(end_x), layout.waveform_plot.max.y),
    )
    .contains(point)
}

/// Return the expanded hit rect for the playback-selection drag handle.
pub(super) fn waveform_selection_drag_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
) -> Option<UiRect> {
    let selection = model.waveform.selection_milli?;
    let start_micros = selection.start_micros.min(selection.end_micros);
    let end_micros = selection.start_micros.max(selection.end_micros);
    if end_micros <= start_micros {
        return None;
    }
    let start_x = waveform_x_for_micros(layout.waveform_plot, model, start_micros);
    let end_x = waveform_x_for_micros(layout.waveform_plot, model, end_micros);
    let selection_rect = UiRect::from_min_max(
        Point::new(start_x.min(end_x), layout.waveform_plot.min.y),
        Point::new(start_x.max(end_x), layout.waveform_plot.max.y),
    );
    let handle = waveform_selection_drag_handle_rect(selection_rect);
    let hit_min = Point::new(
        (handle.min.x - WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).max(layout.waveform_plot.min.x),
        (handle.min.y - WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).max(layout.waveform_plot.min.y),
    );
    let hit_max = Point::new(
        (handle.max.x + WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).min(layout.waveform_plot.max.x),
        (handle.max.y + WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).min(layout.waveform_plot.max.y),
    );
    Some(UiRect::from_min_max(hit_min, hit_max))
}

/// Return the expanded hit rect for one bottom-center selection shift handle.
pub(super) fn waveform_selection_shift_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
    selection: NormalizedRange,
) -> Option<UiRect> {
    let start_micros = selection.start_micros.min(selection.end_micros);
    let end_micros = selection.start_micros.max(selection.end_micros);
    if end_micros <= start_micros {
        return None;
    }
    let selection_rect = UiRect::from_min_max(
        Point::new(
            waveform_x_for_micros(layout.waveform_plot, model, start_micros),
            layout.waveform_plot.min.y,
        ),
        Point::new(
            waveform_x_for_micros(layout.waveform_plot, model, end_micros),
            layout.waveform_plot.max.y,
        ),
    );
    let handle = waveform_selection_shift_handle_rect(selection_rect);
    Some(UiRect::from_min_max(
        Point::new(
            (handle.min.x - WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .max(layout.waveform_plot.min.x),
            (handle.min.y - WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .max(layout.waveform_plot.min.y),
        ),
        Point::new(
            (handle.max.x + WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .min(layout.waveform_plot.max.x),
            (handle.max.y + WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .min(layout.waveform_plot.max.y),
        ),
    ))
}

/// Return the visible playback-selection drag handle rect.
fn waveform_selection_drag_handle_rect(selection_rect: UiRect) -> UiRect {
    let size = WAVEFORM_SELECTION_DRAG_HANDLE_SIZE
        .min(selection_rect.width().max(f32::EPSILON))
        .min(selection_rect.height().max(f32::EPSILON));
    let min = Point::new(selection_rect.max.x - size, selection_rect.max.y - size);
    UiRect::from_min_max(min, selection_rect.max)
}

/// Return the visible bottom-center selection shift handle rect.
fn waveform_selection_shift_handle_rect(selection_rect: UiRect) -> UiRect {
    let width = WAVEFORM_SELECTION_SHIFT_HANDLE_WIDTH.min(selection_rect.width().max(f32::EPSILON));
    let height =
        WAVEFORM_SELECTION_SHIFT_HANDLE_HEIGHT.min(selection_rect.height().max(f32::EPSILON));
    let max_left = (selection_rect.max.x - width).max(selection_rect.min.x);
    let left = (selection_rect.min.x + (selection_rect.width() - width) * 0.5)
        .clamp(selection_rect.min.x, max_left);
    let top = (selection_rect.max.y - height).max(selection_rect.min.y);
    UiRect::from_min_max(
        Point::new(left, top),
        Point::new(
            (left + width).min(selection_rect.max.x),
            selection_rect.max.y,
        ),
    )
}

/// Return the visible square top-tab hit rect for one edit-fade handle.
fn waveform_edit_fade_top_handle_hit_rect(
    waveform_plot: UiRect,
    selection_rect: UiRect,
    x: f32,
) -> UiRect {
    let size = WAVEFORM_EDIT_FADE_TOP_TAB_SIZE
        .max(WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH)
        .min(selection_rect.height().max(1.0))
        .min(waveform_plot.width().max(1.0));
    let half = size * 0.5;
    let left = (x - half).clamp(waveform_plot.min.x, waveform_plot.max.x - size.max(1.0));
    let right = (left + size).min(waveform_plot.max.x).max(left + 1.0);
    let bottom = (selection_rect.min.y + size)
        .min(selection_rect.max.y)
        .max(selection_rect.min.y + 1.0);
    UiRect::from_min_max(
        Point::new(left, selection_rect.min.y),
        Point::new(right, bottom),
    )
}
