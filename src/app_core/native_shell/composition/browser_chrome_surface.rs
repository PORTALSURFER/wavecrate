//! Generic browser-chrome surface projection for the native-shell compat path.
//!
//! This module keeps the browser tabs and toolbar strip on the same public
//! `radiant::layout`, `radiant::runtime`, and `radiant::widgets` hosting
//! pattern used by the other post-pilot chrome slices while browser rows,
//! virtualization, and row-level hit testing remain on the compatibility path.

#[path = "browser_chrome_surface_helpers.rs"]
mod helpers;
#[cfg(test)]
#[path = "browser_chrome_surface_tests.rs"]
mod tests;

use super::{style::SizingTokens, widget_nodes::button_node};
use crate::{
    app::AppModel,
    gui::types::{Point, Rect},
    layout::{
        Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, MainAlign, OverflowPolicy,
        SizeModeCross, SizeModeMain, SlotParams, layout_tree,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface},
};
use helpers::{
    BrowserToolbarSurfaceWidths, browser_sort_label, browser_toolbar_surface_widths,
    build_toolbar_children,
};

const TABS_ROOT_ID: u64 = 1200;
const TABS_ITEMS_ID: u64 = 1202;
const TABS_MAP_ID: u64 = 1203;

const TOOLBAR_ROOT_ID: u64 = 1240;
const TOOLBAR_ROW_ID: u64 = 1241;
const TOOLBAR_RATING_BASE_ID: u64 = 1250;
const TOOLBAR_PLAYBACK_BASE_ID: u64 = 1260;
const TOOLBAR_MARKED_ID: u64 = 1270;
const TOOLBAR_DERIVED_LABEL_ID: u64 = 1271;
const TOOLBAR_RANDOM_ID: u64 = 1272;
const TOOLBAR_CLEANUP_ID: u64 = 1273;
const TOOLBAR_SEARCH_ID: u64 = 1274;
const TOOLBAR_TAGS_ID: u64 = 1275;
const TOOLBAR_ACTIVITY_ID: u64 = 1276;
const TOOLBAR_SORT_ID: u64 = 1277;
const TOOLBAR_TRIAGE_BASE_ID: u64 = 1280;

const BROWSER_RATING_FILTER_COUNT: usize = 8;
const BROWSER_PLAYBACK_AGE_FILTER_COUNT: usize = 3;
const BROWSER_TRIAGE_CHIP_COUNT: usize = 3;

/// User-facing tab labels projected into the generic browser tabs surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserTabsSurfaceContent {
    /// Items-tab label shown on the left tab.
    pub items_label: String,
    /// Map-tab label shown on the right tab.
    pub map_label: String,
}

/// Resolved widget bounds for the generic browser tabs surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserTabsSurfaceLayout {
    /// Items-tab button bounds.
    pub items: Rect,
    /// Map-tab button bounds.
    pub map: Rect,
}

/// User-facing toolbar labels projected into the generic browser toolbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserToolbarSurfaceContent {
    /// Search-field value.
    pub search_value: String,
    /// Search-field placeholder text.
    pub search_placeholder: String,
    /// Activity chip label.
    pub activity_label: String,
    /// Sort chip label.
    pub sort_label: String,
}

/// Resolved widget bounds for the generic browser toolbar surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserToolbarSurfaceLayout {
    /// Rating-filter chip bounds from left to right.
    pub rating_filter_chips: [Rect; BROWSER_RATING_FILTER_COUNT],
    /// Playback-age chip bounds from left to right.
    pub playback_age_filter_chips: [Rect; BROWSER_PLAYBACK_AGE_FILTER_COUNT],
    /// Marked-filter chip bounds.
    pub marked_filter_chip: Rect,
    /// Derived-label-filter chip bounds.
    pub derived_label_filter_chip: Rect,
    /// Toolbar action button bounds in `Random`, `Cleanup`, `Tags` order.
    pub action_slots: [Rect; 3],
    /// Search-field bounds.
    pub search_field: Rect,
    /// Activity-chip bounds.
    pub activity_chip: Rect,
    /// Sort-chip bounds.
    pub sort_chip: Rect,
    /// Reserved triage-chip bounds.
    pub triage_chips: [Rect; BROWSER_TRIAGE_CHIP_COUNT],
}

/// Build user-facing browser-tab content from the projected app model.
pub(crate) fn browser_tabs_surface_content(model: &AppModel) -> BrowserTabsSurfaceContent {
    BrowserTabsSurfaceContent {
        items_label: format!(
            "{} ({})",
            model.browser_chrome.items_tab_label,
            model
                .columns
                .get(1)
                .map(|column| column.item_count)
                .unwrap_or(0)
        ),
        map_label: model.browser_chrome.map_tab_label.clone(),
    }
}

/// Build user-facing browser-toolbar content from the projected app model.
pub(crate) fn browser_toolbar_surface_content(model: &AppModel) -> BrowserToolbarSurfaceContent {
    BrowserToolbarSurfaceContent {
        search_value: model.browser.search_query.clone(),
        search_placeholder: model.browser_chrome.search_placeholder.clone(),
        activity_label: if model.browser.busy {
            model.browser_chrome.activity_busy_label.clone()
        } else {
            model.browser_chrome.activity_ready_label.clone()
        },
        sort_label: browser_sort_label(model),
    }
}

/// Resolve the generic browser tabs surface layout inside one shell band.
pub(crate) fn resolve_browser_tabs_surface_layout(
    tabs_rect: Rect,
    sizing: SizingTokens,
    content: &BrowserTabsSurfaceContent,
) -> BrowserTabsSurfaceLayout {
    let surface = build_browser_tabs_surface(content, sizing, tabs_rect.width());
    let output = layout_tree(&surface.layout_node(), tabs_rect);
    let empty = Rect::from_min_max(tabs_rect.min, tabs_rect.min);
    BrowserTabsSurfaceLayout {
        items: clamp_rect_to_bounds(rect_for(&output.rects, TABS_ITEMS_ID, empty), tabs_rect),
        map: clamp_rect_to_bounds(rect_for(&output.rects, TABS_MAP_ID, empty), tabs_rect),
    }
}

/// Resolve the generic browser-toolbar surface layout inside one shell band.
pub(crate) fn resolve_browser_toolbar_surface_layout(
    toolbar_rect: Rect,
    sizing: SizingTokens,
    content: &BrowserToolbarSurfaceContent,
) -> BrowserToolbarSurfaceLayout {
    let widths = browser_toolbar_surface_widths(toolbar_rect, sizing);
    let surface = build_browser_toolbar_surface(content, toolbar_rect.height(), widths);
    let output = layout_tree(&surface.layout_node(), toolbar_rect);
    let empty = Rect::from_min_max(toolbar_rect.min, toolbar_rect.min);
    BrowserToolbarSurfaceLayout {
        rating_filter_chips: std::array::from_fn(|index| {
            clamp_rect_to_bounds(
                rect_for(&output.rects, TOOLBAR_RATING_BASE_ID + index as u64, empty),
                toolbar_rect,
            )
        }),
        playback_age_filter_chips: std::array::from_fn(|index| {
            clamp_rect_to_bounds(
                rect_for(
                    &output.rects,
                    TOOLBAR_PLAYBACK_BASE_ID + index as u64,
                    empty,
                ),
                toolbar_rect,
            )
        }),
        marked_filter_chip: clamp_rect_to_bounds(
            rect_for(&output.rects, TOOLBAR_MARKED_ID, empty),
            toolbar_rect,
        ),
        derived_label_filter_chip: clamp_rect_to_bounds(
            rect_for(&output.rects, TOOLBAR_DERIVED_LABEL_ID, empty),
            toolbar_rect,
        ),
        action_slots: [
            clamp_rect_to_bounds(
                rect_for(&output.rects, TOOLBAR_RANDOM_ID, empty),
                toolbar_rect,
            ),
            clamp_rect_to_bounds(
                rect_for(&output.rects, TOOLBAR_CLEANUP_ID, empty),
                toolbar_rect,
            ),
            clamp_rect_to_bounds(
                rect_for(&output.rects, TOOLBAR_TAGS_ID, empty),
                toolbar_rect,
            ),
        ],
        search_field: clamp_rect_to_bounds(
            rect_for(&output.rects, TOOLBAR_SEARCH_ID, empty),
            toolbar_rect,
        ),
        activity_chip: clamp_rect_to_bounds(
            rect_for(&output.rects, TOOLBAR_ACTIVITY_ID, empty),
            toolbar_rect,
        ),
        sort_chip: clamp_rect_to_bounds(
            rect_for(&output.rects, TOOLBAR_SORT_ID, empty),
            toolbar_rect,
        ),
        triage_chips: std::array::from_fn(|index| {
            clamp_rect_to_bounds(
                rect_for(&output.rects, TOOLBAR_TRIAGE_BASE_ID + index as u64, empty),
                toolbar_rect,
            )
        }),
    }
}

fn build_browser_tabs_surface(
    content: &BrowserTabsSurfaceContent,
    sizing: SizingTokens,
    band_width: f32,
) -> UiSurface<()> {
    let tab_min_width = 64.0_f32.min(band_width.max(0.0));
    UiSurface::new(SurfaceNode::container(
        TABS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: sizing.action_button_gap.max(1.0),
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SurfaceChild::new(
                fill_slot(tab_min_width),
                button_node(TABS_ITEMS_ID, &content.items_label, 1.0, 1.0),
            ),
            SurfaceChild::new(
                fill_slot(tab_min_width),
                button_node(TABS_MAP_ID, &content.map_label, 1.0, 1.0),
            ),
        ],
    ))
}

fn build_browser_toolbar_surface(
    content: &BrowserToolbarSurfaceContent,
    band_height: f32,
    widths: BrowserToolbarSurfaceWidths,
) -> UiSurface<()> {
    let search_height = band_height.max(1.0);
    let chip_label_height = widths.filter_side.max(1.0);
    UiSurface::new(SurfaceNode::container(
        TOOLBAR_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: widths.horizontal_padding.max(0.0),
                right: widths.horizontal_padding.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                TOOLBAR_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: 0.0,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Center,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                build_toolbar_children(content, search_height, chip_label_height, widths),
            ),
        )],
    ))
}

fn fill_slot(min_width: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fill(1.0),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(min_width, f32::INFINITY, 0.0, f32::INFINITY),
        margin: Insets::default(),
        align_cross_override: Some(CrossAlign::Stretch),
        allow_fixed_compress: false,
    }
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}
