//! Slotized geometry helpers for browser tab surfaces.
#![allow(dead_code)]

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Rect, Vector2};

const BROWSER_TABS_ROOT_ID: u64 = 1770;
const BROWSER_TABS_ITEMS_ID: u64 = 1771;
const BROWSER_TABS_MAP_ID: u64 = 1772;

/// Slot-resolved browser tab button rects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserTabsRects {
    pub items: Rect,
    pub map: Rect,
}

/// Compute browser tab button geometry via strict slotized row layout.
pub(crate) fn compute_browser_tabs_rects(
    tabs_rect: Rect,
    sizing: SizingTokens,
) -> BrowserTabsRects {
    let empty = empty_rect(tabs_rect);
    if tabs_rect.width() <= 0.0 || tabs_rect.height() <= 0.0 {
        return BrowserTabsRects {
            items: empty,
            map: empty,
        };
    }
    let tab_min_width = 64.0_f32.min(tabs_rect.width().max(0.0));
    let tree = LayoutNode::container(
        BROWSER_TABS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: sizing.action_button_gap.max(1.0),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            fill_tab_slot(BROWSER_TABS_ITEMS_ID, tab_min_width),
            fill_tab_slot(BROWSER_TABS_MAP_ID, tab_min_width),
        ],
    );
    let output = layout_tree(&tree, tabs_rect);
    BrowserTabsRects {
        items: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_TABS_ITEMS_ID, empty),
            tabs_rect,
        ),
        map: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_TABS_MAP_ID, empty),
            tabs_rect,
        ),
    }
}

fn fill_tab_slot(node_id: u64, min_width: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fill(1.0),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(min_width, f32::INFINITY, 0.0, f32::INFINITY),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, 1.0)),
    }
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    rect.clamp_to(bounds)
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    bounds.empty_at_min()
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
    fn browser_tabs_rects_stay_inside_tabs_band() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let tabs = Rect::from_min_max(
            crate::gui::types::Point::new(220.0, 244.0),
            crate::gui::types::Point::new(1580.0, 276.0),
        );
        let rects = compute_browser_tabs_rects(tabs, style.sizing);
        assert_inside(tabs, rects.items);
        assert_inside(tabs, rects.map);
        assert!(rects.items.max.x <= rects.map.min.x);
    }

    #[test]
    fn browser_tabs_rects_split_evenly_for_wide_band() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let tabs = Rect::from_min_max(
            crate::gui::types::Point::new(220.0, 244.0),
            crate::gui::types::Point::new(1220.0, 276.0),
        );
        let rects = compute_browser_tabs_rects(tabs, style.sizing);
        let diff = (rects.items.width() - rects.map.width()).abs();
        assert!(diff <= 1.0);
    }

    #[test]
    fn browser_tabs_rects_collapse_for_empty_band() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let tabs = Rect::from_min_max(
            crate::gui::types::Point::new(220.0, 244.0),
            crate::gui::types::Point::new(220.0, 244.0),
        );
        let rects = compute_browser_tabs_rects(tabs, style.sizing);
        assert_eq!(rects.items, tabs);
        assert_eq!(rects.map, tabs);
    }
}
