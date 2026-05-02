//! Slotized sidebar row and recovery-badge text-line geometry helpers.

use super::super::style::SizingTokens;
use crate::gui::text_layout::{TextLineInsets, centered_text_line};
use crate::gui::types::{Point, Rect};

const SIDEBAR_TEXT_LINE_ID: u64 = 1702;
const SIDEBAR_BADGE_TEXT_ID: u64 = 1710;

/// Shared geometry for one sidebar folder row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarFolderRowLayout {
    /// Horizontal indent reserved by tree depth before the disclosure gutter.
    pub depth_indent: f32,
    /// Clickable disclosure gutter reserved ahead of the label.
    pub disclosure_rect: Rect,
    /// Text bounds for the folder label.
    pub label_rect: Rect,
}

/// Compute source-row label bounds through strict slotized text layout.
pub(crate) fn compute_sidebar_source_row_text_rect(row_rect: Rect, sizing: SizingTokens) -> Rect {
    let inset = sizing.text_inset_x + sizing.row_corner_inset;
    let bounds = inset_rect_horizontal(row_rect, inset, inset);
    compute_sidebar_text_line(
        bounds,
        sizing.font_body,
        sizing.text_inset_y,
        SIDEBAR_TEXT_LINE_ID,
    )
}

/// Compute the horizontal indent for one tree depth level.
pub(crate) fn compute_sidebar_folder_row_depth_indent(
    row_rect: Rect,
    sizing: SizingTokens,
    depth: usize,
) -> f32 {
    (depth as f32 * sizing.folder_indent_step).min((row_rect.width() * 0.45).max(0.0))
}

/// Compute shared disclosure-gutter and label bounds for one folder row.
pub(crate) fn compute_sidebar_folder_row_layout(
    row_rect: Rect,
    sizing: SizingTokens,
    depth_indent: f32,
) -> SidebarFolderRowLayout {
    let base_inset = sizing.text_inset_x + sizing.row_corner_inset;
    let depth_indent = depth_indent.max(0.0);
    let gutter_width = folder_row_disclosure_gutter_width(sizing);
    let gutter_spacing = folder_row_disclosure_spacing(sizing);
    let disclosure_left = row_rect.min.x + base_inset + depth_indent;
    let disclosure_right = (disclosure_left + gutter_width).min(row_rect.max.x);
    let disclosure_rect = Rect::from_min_max(
        Point::new(disclosure_left.min(row_rect.max.x), row_rect.min.y),
        Point::new(disclosure_right, row_rect.max.y.max(row_rect.min.y)),
    );
    let left_inset = base_inset + depth_indent + gutter_width + gutter_spacing;
    let bounds = inset_rect_horizontal(row_rect, left_inset, base_inset);
    let label_rect = compute_sidebar_text_line(
        bounds,
        sizing.font_body,
        sizing.text_inset_y,
        SIDEBAR_TEXT_LINE_ID + 1,
    );
    SidebarFolderRowLayout {
        depth_indent,
        disclosure_rect,
        label_rect,
    }
}

/// Compute recovery-badge label bounds through strict slotized text layout.
pub(crate) fn compute_sidebar_recovery_badge_text_rect(
    badge_rect: Rect,
    sizing: SizingTokens,
) -> Rect {
    let inset = sizing.text_inset_x.max(0.0);
    let bounds = inset_rect_horizontal(badge_rect, inset, inset);
    compute_sidebar_text_line(
        bounds,
        sizing.font_meta,
        sizing.text_inset_y,
        SIDEBAR_BADGE_TEXT_ID,
    )
}

fn compute_sidebar_text_line(rect: Rect, font_size: f32, inset_y: f32, node_seed: u64) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    centered_text_line(
        rect,
        font_size,
        TextLineInsets::horizontal(0.0),
        inset_y.max(0.0),
        SIDEBAR_TEXT_LINE_ID + node_seed,
    )
}

fn inset_rect_horizontal(rect: Rect, left: f32, right: f32) -> Rect {
    let min_x = (rect.min.x + left.max(0.0)).min(rect.max.x);
    let max_x = (rect.max.x - right.max(0.0)).max(min_x);
    Rect::from_min_max(
        Point::new(min_x, rect.min.y),
        Point::new(max_x, rect.max.y.max(rect.min.y)),
    )
}

fn folder_row_disclosure_gutter_width(sizing: SizingTokens) -> f32 {
    sizing
        .folder_indent_step
        .max((sizing.font_body * 0.95).ceil())
}

fn folder_row_disclosure_spacing(sizing: SizingTokens) -> f32 {
    (sizing.text_inset_x * 0.4).max(4.0)
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
    fn source_row_text_rect_stays_inside_row() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let row = Rect::from_min_max(Point::new(8.0, 64.0), Point::new(198.0, 80.0));
        let text_rect = compute_sidebar_source_row_text_rect(row, style.sizing);
        assert_inside(row, text_rect);
    }

    #[test]
    fn folder_row_text_rect_respects_depth_indent() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let row = Rect::from_min_max(Point::new(8.0, 296.0), Point::new(198.0, 312.0));
        let depth_indent = 18.0;
        let text_rect =
            compute_sidebar_folder_row_layout(row, style.sizing, depth_indent).label_rect;
        assert_inside(row, text_rect);
        let base_left = row.min.x + style.sizing.text_inset_x + style.sizing.row_corner_inset;
        assert!(text_rect.min.x >= base_left + depth_indent);
    }

    #[test]
    fn recovery_badge_text_rect_stays_inside_badge() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let badge = Rect::from_min_max(Point::new(152.0, 276.0), Point::new(196.0, 292.0));
        let text_rect = compute_sidebar_recovery_badge_text_rect(badge, style.sizing);
        assert_inside(badge, text_rect);
    }
}
