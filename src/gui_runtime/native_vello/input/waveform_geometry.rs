//! Waveform coordinate conversion and normalized-view helpers.

use super::*;
use crate::gui::range::NormalizedPixelSnap;
use crate::gui::visualization::TimelineCoordinateMapper;

/// Absolute waveform position resolved from one pointer point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct WaveformPointerPosition {
    pub(super) plot_ratio: f32,
    pub(super) normalized_ratio: f64,
    pub(super) position_micros: u32,
    pub(super) position_nanos: u32,
}

/// Resolve one absolute waveform pointer position from an arbitrary point.
pub(super) fn waveform_pointer_position_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> WaveformPointerPosition {
    let view = model.waveform.viewport().normalized_viewport();
    let plot_ratio = waveform_ratio_from_point(layout, point);
    let normalized_ratio =
        (view.start_ratio + (view.width_ratio * f64::from(plot_ratio))).clamp(0.0, 1.0);
    WaveformPointerPosition {
        plot_ratio,
        normalized_ratio,
        position_micros: ratio_to_micros(normalized_ratio as f32),
        position_nanos: ratio_to_nanos(normalized_ratio),
    }
}

/// Resolve normalized waveform milli position from an arbitrary pointer point.
#[cfg(test)]
pub(super) fn waveform_position_milli_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u16 {
    ratio_to_milli(
        waveform_pointer_position_from_point(layout, model, point).normalized_ratio as f32,
    )
}

/// Resolve normalized waveform micro position from an arbitrary pointer point.
pub(super) fn waveform_position_micros_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u32 {
    waveform_pointer_position_from_point(layout, model, point).position_micros
}

/// Resolve normalized waveform nanounit position from an arbitrary pointer point.
pub(super) fn waveform_position_nanos_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u32 {
    waveform_pointer_position_from_point(layout, model, point).position_nanos
}

/// Resolve pointer x-position as a normalized ratio within the current plot.
pub(super) fn waveform_ratio_from_point(layout: &ShellLayout, point: Point) -> f32 {
    let inner = layout.waveform_plot;
    let width = inner.width().max(1.0);
    let clamped_x = point.x.clamp(inner.min.x, inner.max.x);
    ((clamped_x - inner.min.x) / width).clamp(0.0, 1.0)
}

/// Return whether the pointer lies horizontally outside the waveform plot.
pub(super) fn waveform_point_is_outside_plot_x(layout: &ShellLayout, point: Point) -> bool {
    point.x < layout.waveform_plot.min.x || point.x > layout.waveform_plot.max.x
}

pub(super) fn ratio_to_milli(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert one normalized view ratio to deterministic micro-units.
pub(super) fn ratio_to_micros(ratio: f32) -> u32 {
    (ratio.clamp(0.0, 1.0) * WAVEFORM_ANCHOR_RATIO_MICROS_SCALE as f32).round() as u32
}

/// Convert one normalized view ratio to deterministic nanounits.
pub(super) fn ratio_to_nanos(ratio: f64) -> u32 {
    (ratio.clamp(0.0, 1.0) * 1_000_000_000.0).round() as u32
}

pub(super) fn micros_from_milli(value: u16) -> u32 {
    u32::from(value.min(1000)) * 1000
}

pub(super) fn waveform_anchor_micros(model: &AppModel) -> u32 {
    model
        .waveform
        .selection_milli
        .map(|selection| selection.start_micros)
        .or(model.waveform.cursor_milli.map(micros_from_milli))
        .or(model.waveform.playhead_milli.map(micros_from_milli))
        .unwrap_or(0)
}

/// Shift one micro-based waveform range while preserving width and clamping to bounds.
#[cfg(test)]
pub(super) fn shift_waveform_range_micros(
    pointer_micros: u32,
    position_micros: u32,
    start_micros: u32,
    end_micros: u32,
) -> (u32, u32) {
    let original_start = i64::from(start_micros.min(end_micros));
    let original_end = i64::from(start_micros.max(end_micros));
    let width = original_end - original_start;
    if width <= 0 {
        return (start_micros, end_micros);
    }
    let delta = i64::from(position_micros) - i64::from(pointer_micros);
    let shifted_start = (original_start + delta).clamp(0, 1_000_000 - width);
    let shifted_end = shifted_start + width;
    (shifted_start as u32, shifted_end as u32)
}

/// Shift one nano-based waveform range while preserving width and clamping to bounds.
pub(super) fn shift_waveform_range_nanos(
    pointer_nanos: u32,
    position_nanos: u32,
    start_nanos: u32,
    end_nanos: u32,
) -> (u32, u32) {
    let original_start = i64::from(start_nanos.min(end_nanos));
    let original_end = i64::from(start_nanos.max(end_nanos));
    let width = original_end - original_start;
    if width <= 0 {
        return (start_nanos, end_nanos);
    }
    let delta = i64::from(position_nanos) - i64::from(pointer_nanos);
    let shifted_start = (original_start + delta).clamp(0, 1_000_000_000 - width);
    let shifted_end = shifted_start + width;
    (shifted_start as u32, shifted_end as u32)
}

pub(super) fn nanos_to_micros(value_nanos: u32) -> u32 {
    ((value_nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000)
}

/// Convert a normalized waveform micro position into plot-space x.
pub(super) fn waveform_x_for_micros(plot: UiRect, model: &AppModel, micros: u32) -> f32 {
    TimelineCoordinateMapper::new(
        model.waveform.viewport(),
        plot,
        NormalizedPixelSnap::Nearest,
    )
    .x_for_micros(micros)
}

/// Return the centered vertical hit span used by waveform resize edges.
pub(super) fn waveform_centered_resize_edge_y_bounds(plot: UiRect) -> (f32, f32) {
    let height = (plot.height() * WAVEFORM_RESIZE_EDGE_HEIGHT_RATIO)
        .max(1.0)
        .min(plot.height());
    let center_y = plot.center().y;
    let top = (center_y - (height * 0.5)).max(plot.min.y);
    let bottom = (top + height).min(plot.max.y).max(top + 1.0);
    (top, bottom)
}

/// Map pointer Y within the waveform plot to one fade-curve milli value.
pub(super) fn waveform_edit_fade_curve_milli_from_point(layout: &ShellLayout, point: Point) -> u16 {
    let plot = layout.waveform_plot;
    let height = plot.height().max(1.0);
    let clamped_y = point.y.clamp(plot.min.y, plot.max.y);
    let ratio = 1.0 - ((clamped_y - plot.min.y) / height).clamp(0.0, 1.0);
    ratio_to_milli(ratio)
}
