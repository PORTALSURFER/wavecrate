//! Slotized browser chrome text-line geometry helpers.

use super::super::style::SizingTokens;
use super::micro_layout::{TextLineInsets, centered_text_line};
use crate::gui::types::Rect;

const TABS_TEXT_SAMPLE_ID: u64 = 1500;
const TABS_TEXT_MAP_ID: u64 = 1501;
const TOOLBAR_TEXT_SEARCH_ID: u64 = 1510;
const TOOLBAR_TEXT_ACTIVITY_ID: u64 = 1511;
const TOOLBAR_TEXT_SORT_ID: u64 = 1512;
const FOOTER_TEXT_SUMMARY_ID: u64 = 1520;

/// Slot-resolved browser-tab label bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserTabsTextLayout {
    pub samples_label: Rect,
    pub map_label: Rect,
}

/// Slot-resolved browser-toolbar chip and field label bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarTextLayout {
    pub search_label: Rect,
    pub activity_label: Rect,
    pub sort_label: Rect,
}

/// Compute browser tab label bounds through strict slotized text-line layout.
pub(crate) fn compute_browser_tabs_text_layout(
    samples_tab: Rect,
    map_tab: Rect,
    sizing: SizingTokens,
) -> BrowserTabsTextLayout {
    BrowserTabsTextLayout {
        samples_label: compute_text_line_rect(
            samples_tab,
            sizing,
            sizing.font_header,
            TABS_TEXT_SAMPLE_ID,
        ),
        map_label: compute_text_line_rect(map_tab, sizing, sizing.font_header, TABS_TEXT_MAP_ID),
    }
}

/// Compute browser toolbar search/activity/sort label bounds.
pub(crate) fn compute_browser_toolbar_text_layout(
    search_field: Rect,
    activity_chip: Rect,
    sort_chip: Rect,
    sizing: SizingTokens,
) -> BrowserToolbarTextLayout {
    BrowserToolbarTextLayout {
        search_label: compute_text_line_rect(
            search_field,
            sizing,
            sizing.font_meta,
            TOOLBAR_TEXT_SEARCH_ID,
        ),
        activity_label: compute_text_line_rect(
            activity_chip,
            sizing,
            sizing.font_meta,
            TOOLBAR_TEXT_ACTIVITY_ID,
        ),
        sort_label: compute_text_line_rect(
            sort_chip,
            sizing,
            sizing.font_meta,
            TOOLBAR_TEXT_SORT_ID,
        ),
    }
}

/// Compute browser footer summary label bounds.
pub(crate) fn compute_browser_footer_text_rect(footer: Rect, sizing: SizingTokens) -> Rect {
    compute_text_line_rect(footer, sizing, sizing.font_meta, FOOTER_TEXT_SUMMARY_ID)
}

fn compute_text_line_rect(rect: Rect, sizing: SizingTokens, font_size: f32, node_id: u64) -> Rect {
    let empty = empty_rect(rect);
    if rect.width() <= 0.0 || rect.height() <= 0.0 || font_size <= 0.0 {
        return empty;
    }
    centered_text_line(
        rect,
        font_size,
        TextLineInsets::symmetric(sizing.text_inset_x.max(0.0), sizing.text_inset_y.max(0.0)),
        0.0,
        node_id,
    )
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;
    use crate::gui::types::Point;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn tabs_text_layout_stays_within_each_tab() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let samples = Rect::from_min_max(Point::new(220.0, 292.0), Point::new(720.0, 320.0));
        let map = Rect::from_min_max(Point::new(724.0, 292.0), Point::new(1220.0, 320.0));
        let layout = compute_browser_tabs_text_layout(samples, map, style.sizing);
        assert_inside(samples, layout.samples_label);
        assert_inside(map, layout.map_label);
    }

    #[test]
    fn toolbar_text_layout_stays_within_toolbar_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let search = Rect::from_min_max(Point::new(220.0, 326.0), Point::new(760.0, 350.0));
        let activity = Rect::from_min_max(Point::new(768.0, 326.0), Point::new(920.0, 350.0));
        let sort = Rect::from_min_max(Point::new(928.0, 326.0), Point::new(1080.0, 350.0));
        let layout = compute_browser_toolbar_text_layout(search, activity, sort, style.sizing);
        assert_inside(search, layout.search_label);
        assert_inside(activity, layout.activity_label);
        assert_inside(sort, layout.sort_label);
    }

    #[test]
    fn footer_text_layout_stays_inside_footer_band() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let footer = Rect::from_min_max(Point::new(220.0, 722.0), Point::new(1220.0, 748.0));
        let line = compute_browser_footer_text_rect(footer, style.sizing);
        assert_inside(footer, line);
    }

    #[test]
    fn toolbar_text_layout_collapses_for_empty_chip() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let search = Rect::from_min_max(Point::new(220.0, 326.0), Point::new(760.0, 350.0));
        let empty = Rect::from_min_max(Point::new(768.0, 326.0), Point::new(768.0, 326.0));
        let layout = compute_browser_toolbar_text_layout(search, empty, empty, style.sizing);
        assert_eq!(layout.activity_label, empty);
        assert_eq!(layout.sort_label, empty);
    }
}
