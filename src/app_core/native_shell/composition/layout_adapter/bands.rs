//! Slotized band-section helpers for native shell layout surfaces.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutDebugOptions,
    LayoutEngine, LayoutNode, LayoutState, MainAlign, OverflowPolicy, SizeModeCross, SizeModeMain,
    SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const TOP_TITLE_CLUSTERS_ROOT_ID: u64 = 610;
const TOP_TITLE_CLUSTERS_ROW_ID: u64 = 611;
const TOP_TITLE_CLUSTER_ID: u64 = 612;
const TOP_ACTION_CLUSTER_ID: u64 = 613;

pub(crate) const BROWSER_BANDS_ROOT_ID: u64 = 620;
const BROWSER_BANDS_COLUMN_ID: u64 = 621;
pub(crate) const BROWSER_TABS_ID: u64 = 622;
pub(crate) const BROWSER_TOOLBAR_ID: u64 = 623;
pub(crate) const BROWSER_HEADER_ID: u64 = 624;
pub(crate) const BROWSER_ROWS_ID: u64 = 625;
pub(crate) const BROWSER_FOOTER_ID: u64 = 626;

/// Slot-resolved top-bar band rectangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TopBarBandSections {
    pub top_bar_title_row: Rect,
    pub top_bar_controls_row: Rect,
    pub top_bar_title_cluster: Rect,
    pub top_bar_action_cluster: Rect,
}

/// Slot-resolved browser band rectangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BrowserBandSections {
    pub browser_tabs: Rect,
    pub browser_toolbar: Rect,
    pub browser_table_header: Rect,
    pub browser_rows: Rect,
    pub browser_footer: Rect,
}

/// Compute top-bar row and cluster bands from strict slot trees.
pub(crate) fn compute_top_bar_band_sections(
    top_bar: Rect,
    sizing: SizingTokens,
) -> TopBarBandSections {
    if top_bar.width() <= 0.0 || top_bar.height() <= 0.0 {
        return TopBarBandSections {
            top_bar_title_row: empty_rect(top_bar),
            top_bar_controls_row: empty_rect(top_bar),
            top_bar_title_cluster: empty_rect(top_bar),
            top_bar_action_cluster: empty_rect(top_bar),
        };
    }
    let title_row = top_bar;
    let controls_row = top_bar;

    let desired_action_cluster_width = ((sizing.action_button_width * 5.0)
        + (sizing.action_button_gap * 4.0)
        + (sizing.text_inset_x * 2.0))
        .clamp(
            sizing.top_bar_action_cluster_min_width,
            sizing.top_bar_action_cluster_max_width,
        );
    let title_inner = inset_horizontal(title_row, sizing.panel_inset);
    let max_action_cluster_width =
        (title_inner.width() - sizing.top_bar_action_cluster_title_reserve_width).max(0.0);
    let action_cluster_width = desired_action_cluster_width
        .min(max_action_cluster_width)
        .max(0.0);

    let clusters_tree = LayoutNode::container(
        TOP_TITLE_CLUSTERS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.panel_inset,
                right: sizing.panel_inset,
                top: 0.0,
                bottom: 0.0,
            },
            align_cross: CrossAlign::Stretch,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                TOP_TITLE_CLUSTERS_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: sizing.top_bar_cluster_gap.max(0.0),
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(
                            TOP_TITLE_CLUSTER_ID,
                            Vector2::new(
                                title_inner.width().max(1.0),
                                title_inner.height().max(1.0),
                            ),
                        ),
                    },
                    SlotChild {
                        slot: fixed_width_slot(action_cluster_width),
                        child: LayoutNode::widget(
                            TOP_ACTION_CLUSTER_ID,
                            Vector2::new(
                                action_cluster_width.max(1.0),
                                title_inner.height().max(1.0),
                            ),
                        ),
                    },
                ],
            ),
        }],
    );
    let cluster_output = layout_tree(&clusters_tree, title_row);
    let title_cluster = clamp_rect_to_bounds(
        rect_for(
            &cluster_output.rects,
            TOP_TITLE_CLUSTER_ID,
            Rect::from_min_max(title_inner.min, title_inner.min),
        ),
        title_row,
    );
    let action_cluster = clamp_rect_to_bounds(
        rect_for(
            &cluster_output.rects,
            TOP_ACTION_CLUSTER_ID,
            Rect::from_min_max(title_inner.max, title_inner.max),
        ),
        title_row,
    );
    TopBarBandSections {
        top_bar_title_row: title_row,
        top_bar_controls_row: controls_row,
        top_bar_title_cluster: title_cluster,
        top_bar_action_cluster: action_cluster,
    }
}

/// Compute browser band sections with a caller-provided persistent layout engine.
pub(crate) fn compute_browser_band_sections_with_layout_engine(
    browser_panel: Rect,
    sizing: SizingTokens,
    engine: &mut LayoutEngine,
    state: &LayoutState,
) -> BrowserBandSections {
    let empty = empty_rect(browser_panel);
    if browser_panel.width() <= 0.0 || browser_panel.height() <= 0.0 {
        return BrowserBandSections {
            browser_tabs: empty,
            browser_toolbar: empty,
            browser_table_header: empty,
            browser_rows: empty,
            browser_footer: empty,
        };
    }
    let band_tree = build_browser_bands_tree(browser_panel, sizing);
    let output = engine.layout_with_state(
        &band_tree,
        browser_panel,
        state,
        LayoutDebugOptions::default(),
    );
    BrowserBandSections {
        browser_tabs: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_TABS_ID, empty),
            browser_panel,
        ),
        browser_toolbar: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_TOOLBAR_ID, empty),
            browser_panel,
        ),
        browser_table_header: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_HEADER_ID, empty),
            browser_panel,
        ),
        browser_rows: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_ROWS_ID, empty),
            browser_panel,
        ),
        browser_footer: clamp_rect_to_bounds(
            rect_for(&output.rects, BROWSER_FOOTER_ID, empty),
            browser_panel,
        ),
    }
}

/// Build browser tabs/toolbar/header/rows/footer tree for persistent runtime caching.
pub(crate) fn build_browser_bands_tree(browser_panel: Rect, sizing: SizingTokens) -> LayoutNode {
    let panel_height = browser_panel.height();
    // Browser bands are framed sections, so they should meet directly with no
    // spacer gutter between them.
    let gap = 0.0;
    let tabs_height = sizing
        .browser_tabs_height
        .max(sizing.browser_tabs_min_height)
        .min(panel_height);
    let toolbar_top = (tabs_height + gap).min(panel_height);
    let toolbar_height = sizing
        .browser_toolbar_height
        .max(sizing.browser_toolbar_min_height)
        .min((panel_height - toolbar_top).max(0.0));
    let header_top = (toolbar_top + toolbar_height + gap).min(panel_height);
    let header_height = sizing
        .browser_table_header_height
        .max(sizing.browser_table_header_min_height)
        .min((panel_height - header_top).max(0.0));
    let footer_height = sizing
        .browser_footer_height
        .clamp(
            sizing.browser_footer_min_height,
            sizing.browser_footer_max_height,
        )
        .min(panel_height);
    LayoutNode::container(
        BROWSER_BANDS_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets::default(),
            align_cross: CrossAlign::Stretch,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                BROWSER_BANDS_COLUMN_ID,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    fixed_height_child(BROWSER_TABS_ID, tabs_height, gap),
                    fixed_height_child(BROWSER_TOOLBAR_ID, toolbar_height, gap),
                    fixed_height_child(BROWSER_HEADER_ID, header_height, gap),
                    SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(BROWSER_ROWS_ID, Vector2::new(1.0, 1.0)),
                    },
                    fixed_height_child(BROWSER_FOOTER_ID, footer_height, 0.0),
                ],
            ),
        }],
    )
}

fn fixed_height_child(node_id: u64, height: f32, bottom_margin: f32) -> SlotChild {
    SlotChild {
        slot: fixed_height_slot(height, bottom_margin),
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}

fn fixed_height_slot(height: f32, bottom_margin: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(height.max(0.0)),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, f32::INFINITY, 0.0, height.max(0.0)),
        margin: Insets {
            bottom: bottom_margin.max(0.0),
            ..Insets::default()
        },
        align_cross_override: None,
        allow_fixed_compress: true,
    }
}

fn fixed_width_slot(width: f32) -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Fixed(width.max(0.0)),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, width.max(0.0), 0.0, f32::INFINITY),
        margin: Insets::default(),
        align_cross_override: None,
        allow_fixed_compress: true,
    }
}

fn rect_for(rects: &std::collections::BTreeMap<u64, Rect>, id: u64, fallback: Rect) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let max_inset = (rect.width() * 0.5).max(0.0);
    let inset = inset.min(max_inset).max(0.0);
    Rect::from_min_max(
        Point::new(rect.min.x + inset, rect.min.y),
        Point::new(rect.max.x - inset, rect.max.y),
    )
}
