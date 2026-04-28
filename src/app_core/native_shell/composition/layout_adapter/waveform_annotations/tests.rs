use super::*;

fn assert_inside(outer: Rect, inner: Rect) {
    assert!(inner.min.x >= outer.min.x);
    assert!(inner.min.y >= outer.min.y);
    assert!(inner.max.x <= outer.max.x);
    assert!(inner.max.y <= outer.max.y);
}

#[test]
fn annotation_rects_stay_inside_waveform_plot() {
    let plot = Rect::from_min_max(Point::new(300.0, 120.0), Point::new(1160.0, 320.0));
    let rects = compute_waveform_annotation_rects(
        plot,
        1.5,
        Some(NormalizedRangeModel::new(120, 640)),
        Some(300),
        Some(780),
        0_u32,
        1_000_000_u32,
    );
    assert_inside(plot, rects.selection.expect("selection"));
    assert_inside(plot, rects.cursor.expect("cursor"));
    assert_inside(plot, rects.playhead.expect("playhead"));
}

#[test]
fn marker_rects_clamp_to_plot_edges() {
    let plot = Rect::from_min_max(Point::new(100.0, 80.0), Point::new(300.0, 200.0));
    let left =
        compute_waveform_annotation_rects(plot, 2.0, None, Some(0), None, 0_u32, 1_000_000_u32);
    let right =
        compute_waveform_annotation_rects(plot, 2.0, None, None, Some(1000), 0_u32, 1_000_000_u32);
    assert_eq!(left.cursor.expect("left marker").min.x, plot.min.x);
    assert_eq!(right.playhead.expect("right marker").max.x, plot.max.x);
}

#[test]
fn empty_plot_returns_no_annotation_rects() {
    let plot = Rect::from_min_max(Point::new(10.0, 10.0), Point::new(10.0, 10.0));
    let rects = compute_waveform_annotation_rects(
        plot,
        1.0,
        Some(NormalizedRangeModel::new(100, 200)),
        Some(150),
        Some(200),
        0_u32,
        1_000_000_u32,
    );
    assert_eq!(rects, WaveformAnnotationRects::default());
}

#[test]
fn marker_rects_respect_view_window() {
    let plot = Rect::from_min_max(Point::new(200.0, 80.0), Point::new(1000.0, 220.0));
    let start = compute_waveform_annotation_rects(
        plot,
        2.0,
        None,
        Some(250),
        None,
        250_000_u32,
        750_000_u32,
    );
    let center = compute_waveform_annotation_rects(
        plot,
        2.0,
        None,
        Some(500),
        None,
        250_000_u32,
        750_000_u32,
    );
    let end = compute_waveform_annotation_rects(
        plot,
        2.0,
        None,
        Some(750),
        None,
        250_000_u32,
        750_000_u32,
    );
    assert_eq!(start.cursor.expect("start marker").min.x, plot.min.x);
    let center_marker = center.cursor.expect("center marker");
    assert!((center_marker.min.x - (plot.min.x + (plot.width() * 0.5))).abs() <= 2.0);
    assert_eq!(end.cursor.expect("end marker").max.x, plot.max.x);
}

#[test]
fn selection_rects_use_micro_precision_inside_narrow_view_windows() {
    let plot = Rect::from_min_max(Point::new(100.0, 40.0), Point::new(300.0, 140.0));
    let rects = compute_waveform_annotation_rects(
        plot,
        1.0,
        Some(NormalizedRangeModel::from_micros(500_400, 500_600)),
        None,
        None,
        500_000_u32,
        501_000_u32,
    );

    let selection = rects.selection.expect("selection");
    assert!((selection.min.x - 180.0).abs() <= 1.0);
    assert!((selection.max.x - 220.0).abs() <= 1.0);
}

#[test]
fn slice_preview_rects_preserve_selection_state_and_stay_inside_plot() {
    let plot = Rect::from_min_max(Point::new(100.0, 40.0), Point::new(300.0, 140.0));
    let slices = compute_waveform_slice_preview_rects(
        plot,
        &[
            WaveformSlicePreviewModel {
                range: NormalizedRangeModel::new(100, 220),
                selected: false,
                focused: false,
                marked_for_export: false,
                duplicate_cleanup_candidate: false,
                duplicate_cleanup_exempted: false,
            },
            WaveformSlicePreviewModel {
                range: NormalizedRangeModel::new(500, 700),
                selected: true,
                focused: true,
                marked_for_export: true,
                duplicate_cleanup_candidate: false,
                duplicate_cleanup_exempted: false,
            },
        ],
        0_u32,
        1_000_000_u32,
    );

    assert_eq!(slices.len(), 2);
    assert!(!slices[0].selected);
    assert!(slices[1].selected);
    assert!(!slices[0].focused);
    assert!(slices[1].focused);
    assert!(!slices[0].marked_for_export);
    assert!(slices[1].marked_for_export);
    assert_inside(plot, slices[0].rect);
    assert_inside(plot, slices[1].rect);
}
