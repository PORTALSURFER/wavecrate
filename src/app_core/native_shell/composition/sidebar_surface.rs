//! Generic sidebar chrome surface projections for the native-shell compatibility layer.
//!
//! This module mirrors the top-bar/status-bar migration pattern for the bounded
//! sidebar chrome. Generic Radiant layout containers and public widgets own the
//! header/footer band composition while source rows, folder rows, inline
//! editors, and recovery-list behavior remain on the compatibility path.

#[path = "sidebar_surface_helpers.rs"]
mod helpers;
#[cfg(test)]
#[path = "sidebar_surface_tests.rs"]
mod tests;

use super::style::SizingTokens;
use crate::{
    app::AppModel,
    gui::types::Rect,
    layout::{
        ContainerKind, ContainerPolicy, CrossAlign, Insets, OverflowPolicy, SlotParams, layout_tree,
    },
    runtime::{SurfaceChild, SurfaceNode, UiSurface},
};
use helpers::{
    button_widget, clamp_rect_to_bounds, fixed_slot, fixed_slot_cross_fill, fixed_slot_with_cross,
    footer_action_button_width, header_button_side, rect_for, text_widget,
};

const HEADER_ROOT_ID: u64 = 1120;
const HEADER_ROW_ID: u64 = 1121;
const HEADER_TEXT_COLUMN_ID: u64 = 1122;
const HEADER_TITLE_ID: u64 = 1123;
const HEADER_QUERY_ID: u64 = 1124;
const HEADER_ADD_BUTTON_ID: u64 = 1125;

const FOOTER_ROOT_ID: u64 = 1130;
const FOOTER_COLUMN_ID: u64 = 1131;
const FOOTER_SUMMARY_ID: u64 = 1132;
const FOOTER_PRIMARY_ID: u64 = 1133;
const FOOTER_SECONDARY_ID: u64 = 1134;
const FOOTER_ACTION_ROW_ID: u64 = 1135;
const FOOTER_ACTION_SPACER_ID: u64 = 1136;
const FOOTER_ACTION_BASE_ID: u64 = 1140;

/// User-facing content projected into the sidebar header band.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SidebarHeaderSurfaceContent {
    /// Sources title shown in the sidebar header.
    pub title: String,
    /// Inline search summary shown under the title.
    pub query: String,
}

/// Resolved widget bounds for the generic sidebar header surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SidebarHeaderSurfaceLayout {
    /// Title text bounds.
    pub title_text_rect: Rect,
    /// Query-summary text bounds.
    pub query_text_rect: Rect,
    /// Add-source button bounds when the band is wide enough.
    pub add_button_rect: Option<Rect>,
}

/// One bounded footer action projected into the generic sidebar footer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SidebarFooterActionSpec {
    /// User-facing action label rendered in the footer button.
    pub label: &'static str,
}

/// User-facing content projected into the sidebar footer band.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SidebarFooterSurfaceContent {
    /// Primary footer summary label.
    pub primary_summary: String,
    /// Secondary footer summary label.
    pub secondary_summary: String,
    /// Footer action labels hosted in the bounded action row.
    pub actions: Vec<SidebarFooterActionSpec>,
}

/// Resolved button geometry for one projected sidebar footer action.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SidebarFooterActionLayout {
    /// Stable user-facing action metadata.
    pub spec: SidebarFooterActionSpec,
    /// Resolved button bounds.
    pub rect: Rect,
}

/// Resolved widget bounds for the generic sidebar footer surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SidebarFooterSurfaceLayout {
    /// Primary summary text bounds.
    pub primary_text_rect: Rect,
    /// Secondary summary text bounds.
    pub secondary_text_rect: Rect,
    /// Visible footer action buttons resolved inside the band.
    pub action_buttons: Vec<SidebarFooterActionLayout>,
}

/// Build user-facing header content from the projected app model.
pub(crate) fn sidebar_header_surface_content(model: &AppModel) -> SidebarHeaderSurfaceContent {
    let title = if model.sources.header.is_empty() {
        model.sources_label.clone()
    } else {
        model.sources.header.clone()
    };
    let query = format!(
        "search: {}",
        if model.sources.search_query.is_empty() {
            "—"
        } else {
            model.sources.search_query.as_str()
        }
    );
    SidebarHeaderSurfaceContent { title, query }
}

/// Build user-facing footer summary content from the projected app model.
pub(crate) fn sidebar_footer_surface_content(
    model: &AppModel,
    rendered_sources: usize,
    rendered_folders: usize,
) -> SidebarFooterSurfaceContent {
    let active_pane = model.sources.active_folder_pane_model();
    let primary_summary = if model.sources.rows.len() > rendered_sources {
        format!("+{} more…", model.sources.rows.len() - rendered_sources)
    } else {
        String::new()
    };
    let secondary_summary = if active_pane.folder_rows.len() > rendered_folders {
        format!(
            "folders: +{} more…",
            active_pane.folder_rows.len() - rendered_folders
        )
    } else if active_pane.folder_recovery.entry_count > 0 {
        format!(
            "recovery entries: {}",
            active_pane.folder_recovery.entry_count
        )
    } else {
        String::new()
    };
    SidebarFooterSurfaceContent {
        primary_summary,
        secondary_summary,
        actions: sidebar_footer_action_specs(),
    }
}

/// Return the canonical sidebar footer action labels in display order.
pub(crate) fn sidebar_footer_action_specs() -> Vec<SidebarFooterActionSpec> {
    [
        "New", "Root", "Rename", "Delete", "Restore", "Purge", "Clear",
    ]
    .into_iter()
    .map(|label| SidebarFooterActionSpec { label })
    .collect()
}

/// Resolve the generic sidebar header surface layout inside one header rect.
pub(crate) fn resolve_sidebar_header_surface_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    content: &SidebarHeaderSurfaceContent,
) -> SidebarHeaderSurfaceLayout {
    let surface = build_sidebar_header_surface(content, sizing);
    let output = layout_tree(&surface.layout_node(), header_rect);
    let empty = Rect::from_min_max(header_rect.min, header_rect.min);
    let add_button = rect_for(&output.rects, HEADER_ADD_BUTTON_ID, empty);
    SidebarHeaderSurfaceLayout {
        title_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, HEADER_TITLE_ID, empty),
            header_rect,
        ),
        query_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, HEADER_QUERY_ID, empty),
            header_rect,
        ),
        add_button_rect: (add_button.width() > 0.0 && add_button.height() > 0.0)
            .then_some(clamp_rect_to_bounds(add_button, header_rect)),
    }
}

/// Resolve the generic sidebar footer surface layout inside one footer rect.
pub(crate) fn resolve_sidebar_footer_surface_layout(
    footer_rect: Rect,
    sizing: SizingTokens,
    content: &SidebarFooterSurfaceContent,
) -> SidebarFooterSurfaceLayout {
    let surface = build_sidebar_footer_surface(content, sizing, footer_rect.width());
    let output = layout_tree(&surface.layout_node(), footer_rect);
    let empty = Rect::from_min_max(footer_rect.min, footer_rect.min);
    let action_buttons = content
        .actions
        .iter()
        .enumerate()
        .filter_map(|(index, spec)| {
            let rect = clamp_rect_to_bounds(
                rect_for(&output.rects, FOOTER_ACTION_BASE_ID + index as u64, empty),
                footer_rect,
            );
            (rect.width() > 0.0 && rect.height() > 0.0).then(|| SidebarFooterActionLayout {
                spec: spec.clone(),
                rect,
            })
        })
        .collect();
    SidebarFooterSurfaceLayout {
        primary_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, FOOTER_PRIMARY_ID, empty),
            footer_rect,
        ),
        secondary_text_rect: clamp_rect_to_bounds(
            rect_for(&output.rects, FOOTER_SECONDARY_ID, empty),
            footer_rect,
        ),
        action_buttons,
    }
}

fn build_sidebar_header_surface(
    content: &SidebarHeaderSurfaceContent,
    sizing: SizingTokens,
) -> UiSurface<()> {
    let button_side = header_button_side(sizing);
    UiSurface::new(SurfaceNode::container(
        HEADER_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: (sizing.text_inset_x + sizing.header_label_gutter).max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                HEADER_ROW_ID,
                ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing: sizing.text_inset_x.max(2.0),
                    align_cross: CrossAlign::Center,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(SlotParams::fill(), header_text_column(content, sizing)),
                    SurfaceChild::new(
                        fixed_slot_with_cross(button_side, button_side),
                        button_widget(HEADER_ADD_BUTTON_ID, "+", button_side, button_side),
                    ),
                ],
            ),
        )],
    ))
}

fn build_sidebar_footer_surface(
    content: &SidebarFooterSurfaceContent,
    sizing: SizingTokens,
    footer_width: f32,
) -> UiSurface<()> {
    let button_height = sizing.sidebar_action_button_height.max(1.0);
    let button_width = footer_action_button_width(footer_width, sizing, content.actions.len());
    let mut action_children = Vec::with_capacity(content.actions.len() + 1);
    action_children.push(SurfaceChild::new(
        SlotParams::fill(),
        text_widget(FOOTER_ACTION_SPACER_ID, "", 1.0, 1.0),
    ));
    for (index, action) in content.actions.iter().enumerate() {
        action_children.push(SurfaceChild::new(
            fixed_slot(button_width),
            button_widget(
                FOOTER_ACTION_BASE_ID + index as u64,
                action.label,
                button_width,
                button_height,
            ),
        ));
    }
    UiSurface::new(SurfaceNode::container(
        FOOTER_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.text_inset_x.max(0.0),
                right: sizing.text_inset_x.max(0.0),
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                FOOTER_COLUMN_ID,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: sizing.text_row_gap.max(0.0),
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(SlotParams::fill(), footer_summary_surface(content, sizing)),
                    SurfaceChild::new(
                        fixed_slot_cross_fill(button_height),
                        SurfaceNode::container(
                            FOOTER_ACTION_ROW_ID,
                            ContainerPolicy {
                                kind: ContainerKind::Row,
                                spacing: sizing.sidebar_action_button_gap.max(0.0),
                                align_cross: CrossAlign::Center,
                                overflow: OverflowPolicy::Clip,
                                ..ContainerPolicy::default()
                            },
                            action_children,
                        ),
                    ),
                ],
            ),
        )],
    ))
}

fn header_text_column(
    content: &SidebarHeaderSurfaceContent,
    sizing: SizingTokens,
) -> SurfaceNode<()> {
    SurfaceNode::container(
        HEADER_TEXT_COLUMN_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            spacing: sizing.text_row_gap.max(0.0),
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SurfaceChild::new(
                fixed_slot_cross_fill(sizing.font_header.max(1.0)),
                text_widget(HEADER_TITLE_ID, &content.title, 1.0, sizing.font_header),
            ),
            SurfaceChild::new(
                fixed_slot_cross_fill(sizing.font_meta.max(1.0)),
                text_widget(HEADER_QUERY_ID, &content.query, 1.0, sizing.font_meta),
            ),
        ],
    )
}

fn footer_summary_surface(
    content: &SidebarFooterSurfaceContent,
    sizing: SizingTokens,
) -> SurfaceNode<()> {
    SurfaceNode::container(
        FOOTER_SUMMARY_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                left: sizing.header_label_gutter.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SurfaceChild::new(
            SlotParams::fill(),
            SurfaceNode::container(
                FOOTER_SUMMARY_ID + 1,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: sizing.text_row_gap.max(0.0),
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                vec![
                    SurfaceChild::new(
                        fixed_slot_cross_fill(sizing.font_meta.max(1.0)),
                        text_widget(
                            FOOTER_PRIMARY_ID,
                            &content.primary_summary,
                            1.0,
                            sizing.font_meta,
                        ),
                    ),
                    SurfaceChild::new(
                        fixed_slot_cross_fill(sizing.font_meta.max(1.0)),
                        text_widget(
                            FOOTER_SECONDARY_ID,
                            &content.secondary_summary,
                            1.0,
                            sizing.font_meta,
                        ),
                    ),
                ],
            ),
        )],
    )
}
