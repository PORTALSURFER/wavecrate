//! Slotized waveform annotation geometry for selection, slice previews, cursor, and playhead.

use crate::app::{NormalizedRangeModel, WaveformSlicePreviewModel};
use crate::gui::types::{Point, Rect};

#[cfg(test)]
#[path = "waveform_annotations/tests.rs"]
mod tests;

/// Pixel-snapping policy for waveform x coordinates before rendering or hit-testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaveformPixelSnap {
    /// Snap the x coordinate to the nearest device pixel.
    Nearest,
}

/// Normalized waveform viewport bounds shared by input, rendering, and hit-testing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveformViewWindow {
    /// Normalized start ratio in `0.0..=1.0`.
    pub(crate) start_ratio: f64,
    /// Normalized visible width ratio.
    pub(crate) width_ratio: f64,
}

/// Waveform annotation rectangles resolved from normalized waveform anchors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaveformAnnotationRects {
    pub selection: Option<Rect>,
    pub cursor: Option<Rect>,
    pub playhead: Option<Rect>,
}

/// Rectangle and selection state for one detected waveform slice preview.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WaveformSlicePreviewRects {
    pub rect: Rect,
    pub selected: bool,
    pub focused: bool,
    pub marked_for_export: bool,
    pub duplicate_cleanup_candidate: bool,
    pub duplicate_cleanup_exempted: bool,
}

/// Compute waveform annotation rectangles constrained to the waveform plot.
#[cfg(test)]
pub(crate) fn compute_waveform_annotation_rects(
    waveform_plot: Rect,
    border_width: f32,
    selection: Option<NormalizedRangeModel>,
    cursor_milli: Option<u16>,
    playhead_milli: Option<u16>,
    view_start_micros: impl Into<u32>,
    view_end_micros: impl Into<u32>,
) -> WaveformAnnotationRects {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return WaveformAnnotationRects::default();
    }
    let view = waveform_view_window_from_bounds(
        view_start_micros.into(),
        view_end_micros.into(),
        None,
        None,
    );
    WaveformAnnotationRects {
        selection: selection.and_then(|range| selection_rect(waveform_plot, range, view)),
        cursor: cursor_milli.and_then(|milli| {
            marker_rect(waveform_plot, border_width, u32::from(milli) * 1000, view)
        }),
        playhead: playhead_milli.and_then(|milli| {
            marker_rect(waveform_plot, border_width, u32::from(milli) * 1000, view)
        }),
    }
}

/// Compute waveform annotation rectangles using nanosecond view bounds when available.
pub(crate) fn compute_waveform_annotation_rects_with_nanos(
    waveform_plot: Rect,
    border_width: f32,
    selection: Option<NormalizedRangeModel>,
    cursor_milli: Option<u16>,
    playhead_milli: Option<u16>,
    view_start_micros: impl Into<u32>,
    view_end_micros: impl Into<u32>,
    view_start_nanos: impl Into<u32>,
    view_end_nanos: impl Into<u32>,
) -> WaveformAnnotationRects {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return WaveformAnnotationRects::default();
    }
    let view = waveform_view_window_from_bounds(
        view_start_micros.into(),
        view_end_micros.into(),
        Some(view_start_nanos.into()),
        Some(view_end_nanos.into()),
    );
    WaveformAnnotationRects {
        selection: selection.and_then(|range| selection_rect(waveform_plot, range, view)),
        cursor: cursor_milli.and_then(|milli| {
            marker_rect(waveform_plot, border_width, u32::from(milli) * 1000, view)
        }),
        playhead: playhead_milli.and_then(|milli| {
            marker_rect(waveform_plot, border_width, u32::from(milli) * 1000, view)
        }),
    }
}

/// Compute all slice-preview rectangles constrained to the waveform plot.
pub(crate) fn compute_waveform_slice_preview_rects(
    waveform_plot: Rect,
    slices: &[WaveformSlicePreviewModel],
    view_start_micros: impl Into<u32>,
    view_end_micros: impl Into<u32>,
) -> Vec<WaveformSlicePreviewRects> {
    if waveform_plot.width() <= 0.0 || waveform_plot.height() <= 0.0 {
        return Vec::new();
    }
    let view = waveform_view_window_from_bounds(
        view_start_micros.into(),
        view_end_micros.into(),
        None,
        None,
    );
    slices
        .iter()
        .filter_map(|slice| {
            slice_rect(waveform_plot, slice.range, view).map(|rect| WaveformSlicePreviewRects {
                rect,
                selected: slice.selected,
                focused: slice.focused,
                marked_for_export: slice.marked_for_export,
                duplicate_cleanup_candidate: slice.duplicate_cleanup_candidate,
                duplicate_cleanup_exempted: slice.duplicate_cleanup_exempted,
            })
        })
        .collect()
}

/// Resolve the authoritative waveform view window from micro and optional nano bounds.
pub(crate) fn waveform_view_window_from_bounds(
    view_start_micros: u32,
    view_end_micros: u32,
    view_start_nanos: Option<u32>,
    view_end_nanos: Option<u32>,
) -> WaveformViewWindow {
    normalized_view_window_from_nanos(
        view_start_micros,
        view_end_micros,
        view_start_nanos,
        view_end_nanos,
    )
    .unwrap_or_else(|| normalized_view_window_from_micros(view_start_micros, view_end_micros))
}

/// Convert one absolute waveform micro position into plot-space x.
pub(crate) fn waveform_plot_x_for_micros(
    waveform_plot: Rect,
    micros: u32,
    view: WaveformViewWindow,
    snap: WaveformPixelSnap,
) -> f32 {
    waveform_plot_x_for_absolute_ratio(
        waveform_plot,
        f64::from(micros.min(1_000_000)) / 1_000_000.0,
        view,
        snap,
    )
}

/// Convert one absolute waveform ratio into plot-space x.
pub(crate) fn waveform_plot_x_for_absolute_ratio(
    waveform_plot: Rect,
    absolute_ratio: f64,
    view: WaveformViewWindow,
    snap: WaveformPixelSnap,
) -> f32 {
    let ratio_in_view = if view.width_ratio <= f64::EPSILON {
        0.0
    } else {
        ((absolute_ratio.clamp(0.0, 1.0) - view.start_ratio) / view.width_ratio).clamp(0.0, 1.0)
            as f32
    };
    let raw_x = waveform_plot.min.x + (waveform_plot.width() * ratio_in_view);
    match snap {
        WaveformPixelSnap::Nearest => raw_x.round(),
    }
    .clamp(waveform_plot.min.x, waveform_plot.max.x)
}

fn slice_rect(
    waveform_plot: Rect,
    range: NormalizedRangeModel,
    view: WaveformViewWindow,
) -> Option<Rect> {
    let start = waveform_plot_x_for_micros(
        waveform_plot,
        range.start_micros,
        view,
        WaveformPixelSnap::Nearest,
    );
    let end = waveform_plot_x_for_micros(
        waveform_plot,
        range.end_micros,
        view,
        WaveformPixelSnap::Nearest,
    );
    let left = start
        .min(end)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let right = end
        .max(start)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let expanded_right = right.max((left + 1.0).min(waveform_plot.max.x));
    (expanded_right > left).then_some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(expanded_right, waveform_plot.max.y),
    ))
}

fn selection_rect(
    waveform_plot: Rect,
    selection: NormalizedRangeModel,
    view: WaveformViewWindow,
) -> Option<Rect> {
    let start = waveform_plot_x_for_micros(
        waveform_plot,
        selection.start_micros,
        view,
        WaveformPixelSnap::Nearest,
    );
    let end = waveform_plot_x_for_micros(
        waveform_plot,
        selection.end_micros,
        view,
        WaveformPixelSnap::Nearest,
    );
    let left = start
        .min(end)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let right = end
        .max(start)
        .clamp(waveform_plot.min.x, waveform_plot.max.x);
    let expanded_right = right.max((left + 1.0).min(waveform_plot.max.x));
    (expanded_right > left).then_some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(expanded_right, waveform_plot.max.y),
    ))
}

fn marker_rect(
    waveform_plot: Rect,
    border_width: f32,
    micros: u32,
    view: WaveformViewWindow,
) -> Option<Rect> {
    let marker_width = border_width.max(1.0).min(waveform_plot.width());
    if marker_width <= 0.0 {
        return None;
    }
    let raw_x = waveform_plot_x_for_micros(waveform_plot, micros, view, WaveformPixelSnap::Nearest);
    let left = raw_x.clamp(waveform_plot.min.x, waveform_plot.max.x - marker_width);
    let right = (left + marker_width).min(waveform_plot.max.x);
    (right > left).then_some(Rect::from_min_max(
        Point::new(left, waveform_plot.min.y),
        Point::new(right, waveform_plot.max.y),
    ))
}

fn normalized_view_window_from_micros(
    view_start_micros: u32,
    view_end_micros: u32,
) -> WaveformViewWindow {
    let start_micros = view_start_micros.min(1_000_000);
    let end_micros = view_end_micros.min(1_000_000).max(start_micros);
    let start_ratio = f64::from(start_micros) / 1_000_000.0;
    let width_ratio =
        (f64::from(end_micros.saturating_sub(start_micros)) / 1_000_000.0).max(f64::EPSILON);
    WaveformViewWindow {
        start_ratio,
        width_ratio,
    }
}

fn normalized_view_window_from_nanos(
    view_start_micros: u32,
    view_end_micros: u32,
    view_start_nanos: Option<u32>,
    view_end_nanos: Option<u32>,
) -> Option<WaveformViewWindow> {
    let start_nanos = view_start_nanos?.min(1_000_000_000);
    let end_nanos = view_end_nanos?.min(1_000_000_000).max(start_nanos);
    let start_matches = micros_matches_projected_nanos(view_start_micros, start_nanos);
    let end_matches = micros_matches_projected_nanos(view_end_micros, end_nanos);
    if !start_matches || !end_matches {
        return None;
    }
    Some(WaveformViewWindow {
        start_ratio: f64::from(start_nanos) / 1_000_000_000.0,
        width_ratio: (f64::from(end_nanos.saturating_sub(start_nanos)) / 1_000_000_000.0)
            .max(f64::EPSILON),
    })
}

fn micros_matches_projected_nanos(view_micros: u32, view_nanos: u32) -> bool {
    let projected_micros = ((view_nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000);
    projected_micros.abs_diff(view_micros.min(1_000_000)) <= 1
}
