//! Slotized browser table column, header-label, and row-label geometry helpers.

use super::super::style::SizingTokens;
use crate::gui::text_layout::{TextLineInsets, centered_text_line};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const BROWSER_COLUMNS_ROOT_ID: u64 = 1200;
const BROWSER_COL_INDEX_ID: u64 = 1201;
const BROWSER_COL_SAMPLE_ID: u64 = 1202;
const BROWSER_TEXT_ROOT_ID: u64 = 1210;

/// Slot-resolved browser table columns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserTableColumns {
    pub index: Rect,
    pub sample: Rect,
    pub bucket: Rect,
}

/// Slot-resolved browser table-header labels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserHeaderTextLayout {
    pub columns: BrowserTableColumns,
    pub index_label: Rect,
    pub sample_label: Rect,
    pub bucket_label: Rect,
}

/// Slot-resolved browser row text geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserRowTextLayout {
    pub columns: BrowserTableColumns,
    pub index_label: Rect,
    pub sample_label: Rect,
    pub bucket_chip: Rect,
    pub bucket_label: Rect,
}

/// Compute browser index/sample columns via strict slot layout.
pub(crate) fn compute_browser_table_columns(
    rect: Rect,
    sizing: SizingTokens,
) -> BrowserTableColumns {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return BrowserTableColumns {
            index: empty,
            sample: empty,
            bucket: empty,
        };
    }
    let tree = LayoutNode::container(
        BROWSER_COLUMNS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Row,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            fixed_width_child(BROWSER_COL_INDEX_ID, index_width(rect, sizing)),
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(BROWSER_COL_SAMPLE_ID, Vector2::new(1.0, 1.0)),
            },
        ],
    );
    let output = layout_tree(&tree, rect);
    let index = clamp_rect_to_bounds(rect_for(&output.rects, BROWSER_COL_INDEX_ID, empty), rect);
    let raw_sample =
        clamp_rect_to_bounds(rect_for(&output.rects, BROWSER_COL_SAMPLE_ID, empty), rect);
    let sample_min_x = raw_sample.min.x.max(index.max.x);
    let sample = Rect::from_min_max(
        Point::new(sample_min_x, raw_sample.min.y),
        Point::new(raw_sample.max.x.max(sample_min_x), raw_sample.max.y),
    );
    let collapsed_bucket = Rect::from_min_max(sample.max, sample.max);
    BrowserTableColumns {
        index,
        sample,
        bucket: collapsed_bucket,
    }
}

/// Compute browser table-header label bounds for `#` and the primary item column.
pub(crate) fn compute_browser_header_text_layout(
    header_rect: Rect,
    sizing: SizingTokens,
) -> BrowserHeaderTextLayout {
    let columns = compute_browser_table_columns(header_rect, sizing);
    BrowserHeaderTextLayout {
        columns,
        index_label: compute_text_line_rect(columns.index, sizing, sizing.font_meta),
        sample_label: compute_text_line_rect(columns.sample, sizing, sizing.font_meta),
        bucket_label: compute_text_line_rect(columns.bucket, sizing, sizing.font_meta),
    }
}

/// Compute browser-row index/item label bounds plus collapsed inline-metadata anchors.
pub(crate) fn compute_browser_row_text_layout(
    row_rect: Rect,
    sizing: SizingTokens,
) -> BrowserRowTextLayout {
    let columns = compute_browser_table_columns(row_rect, sizing);
    let bucket_chip = columns.bucket;
    let index_label = snap_browser_row_text_baseline(compute_text_line_rect(
        columns.index,
        sizing,
        sizing.font_meta,
    ));
    let sample_label = snap_browser_row_text_baseline(compute_text_line_rect(
        columns.sample,
        sizing,
        sizing.font_body,
    ));
    let bucket_label = snap_browser_row_text_baseline(compute_text_line_rect(
        bucket_chip,
        sizing,
        sizing.font_meta,
    ));
    BrowserRowTextLayout {
        columns,
        index_label,
        sample_label,
        bucket_chip,
        bucket_label,
    }
}

fn index_width(rect: Rect, sizing: SizingTokens) -> f32 {
    sizing.browser_index_col_width.max(20.0).min(rect.width())
}

fn fixed_width_child(node_id: u64, width: f32) -> SlotChild {
    let width = width.max(0.0);
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(width),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(width, width, 0.0, f32::INFINITY),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(width.max(1.0), 1.0)),
    }
}

fn compute_text_line_rect(rect: Rect, sizing: SizingTokens, font_size: f32) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    centered_text_line(
        rect,
        font_size,
        TextLineInsets::symmetric(sizing.text_inset_x.max(0.0), sizing.text_inset_y.max(0.0)),
        0.0,
        BROWSER_TEXT_ROOT_ID,
    )
}

/// Snap a browser-row label line so the rendered text baseline lands on a full
/// pixel. This keeps repeated table rows from alternating between adjacent
/// raster rows when compact density tokens produce fractional heights.
fn snap_browser_row_text_baseline(line: Rect) -> Rect {
    let height = line.height().max(0.0);
    if height <= 0.0 {
        return line;
    }
    let baseline = (line.min.y + height).round();
    let min_y = baseline - height;
    Rect::from_min_max(
        Point::new(line.min.x, min_y),
        Point::new(line.max.x, baseline),
    )
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn browser_columns_preserve_left_to_right_order() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let rect = Rect::from_min_max(Point::new(100.0, 50.0), Point::new(1120.0, 78.0));
        let columns = compute_browser_table_columns(rect, style.sizing);
        assert_inside(rect, columns.index);
        assert_inside(rect, columns.sample);
        assert_inside(rect, columns.bucket);
        assert!(columns.index.max.x <= columns.sample.min.x);
        assert!(columns.sample.max.x <= columns.bucket.min.x);
    }

    #[test]
    fn browser_header_labels_stay_inside_columns() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let header = Rect::from_min_max(Point::new(80.0, 16.0), Point::new(1240.0, 42.0));
        let layout = compute_browser_header_text_layout(header, style.sizing);
        assert_inside(layout.columns.index, layout.index_label);
        assert_inside(layout.columns.sample, layout.sample_label);
        assert_inside(layout.columns.bucket, layout.bucket_label);
    }

    #[test]
    fn browser_row_chip_and_labels_stay_inside_row() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let row = Rect::from_min_max(Point::new(200.0, 90.0), Point::new(1200.0, 118.0));
        let layout = compute_browser_row_text_layout(row, style.sizing);
        assert_inside(row, layout.columns.index);
        assert_inside(row, layout.columns.sample);
        assert_inside(row, layout.columns.bucket);
        assert_inside(layout.columns.bucket, layout.bucket_chip);
        assert_inside(layout.columns.index, layout.index_label);
        assert_inside(layout.columns.sample, layout.sample_label);
        assert_inside(layout.bucket_chip, layout.bucket_label);
    }

    #[test]
    fn browser_row_labels_snap_baselines_to_pixels() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let row = Rect::from_min_max(
            Point::new(200.0, 90.0),
            Point::new(1200.0, 90.0 + style.sizing.browser_row_height),
        );
        let layout = compute_browser_row_text_layout(row, style.sizing);
        for label in [layout.index_label, layout.sample_label, layout.bucket_label] {
            let baseline = label.min.y + label.height();
            assert!(
                (baseline - baseline.round()).abs() <= 0.001,
                "baseline {baseline} should align to the pixel grid"
            );
        }
    }
}
