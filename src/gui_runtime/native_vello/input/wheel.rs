//! Wheel-to-action mapping for browser-list scrolling and timeline zoom.

use super::*;
use crate::gui::list::{
    virtual_list_scroll_delta_from_units, virtual_list_view_start_after_scroll_delta,
};

pub(super) fn browser_list_wheel_row_delta(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    if model.map.active || !layout.browser_panel.contains(point) {
        return None;
    }
    let row_stride = (style.sizing.browser_row_height + style.sizing.browser_row_gap).max(1.0);
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => -y,
        MouseScrollDelta::PixelDelta(position) => -(position.y as f32) / row_stride,
    };
    virtual_list_scroll_delta_from_units(raw)
}

pub(super) fn folder_wheel_row_delta(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    if !shell_state.folder_panel_contains_point(layout, model, point) {
        return None;
    }
    let row_stride = (style.sizing.folder_row_height + style.sizing.folder_row_gap).max(1.0);
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => -y,
        MouseScrollDelta::PixelDelta(position) => -(position.y as f32) / row_stride,
    };
    virtual_list_scroll_delta_from_units(raw)
}

/// Clamp one wheel-derived browser-list viewport move to the current visible-row range.
pub(super) fn browser_list_view_start_after_wheel(
    current_view_start: usize,
    visible_count: usize,
    viewport_len: usize,
    steps: i8,
) -> Option<usize> {
    virtual_list_view_start_after_scroll_delta(
        current_view_start,
        visible_count,
        viewport_len,
        steps as isize,
    )
}

/// Map one mouse-wheel delta into timeline zoom action while hovering the timeline card.
pub(super) fn waveform_wheel_zoom_action(
    layout: &ShellLayout,
    _model: &AppModel,
    point: Point,
    delta: MouseScrollDelta,
) -> Option<UiAction> {
    if !layout.waveform_card.contains(point) {
        return None;
    }
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(position) => {
            (position.y as f32) / WAVEFORM_WHEEL_ZOOM_PIXEL_STEP
        }
    };
    if raw.abs() <= f32::EPSILON {
        return None;
    }
    let zoom_in = raw > 0.0;
    let mut steps = raw.abs().round();
    if steps < 1.0 {
        steps = 1.0;
    }
    Some(UiAction::ZoomWaveform {
        zoom_in,
        steps: steps.min(u8::MAX as f32) as u8,
        anchor_ratio_micros: Some(ratio_to_micros(waveform_ratio_from_point(layout, point))),
    })
}
