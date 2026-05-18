//! Slotized sidebar folder-header text/badge/divider micro-layout helpers.

use super::super::style::SizingTokens;
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

const FOLDER_HEADER_TEXT_ROOT_ID: u64 = 1000;
const FOLDER_HEADER_TEXT_COLUMN_ID: u64 = 1001;
const FOLDER_HEADER_TITLE_ID: u64 = 1002;
const FOLDER_HEADER_META_ID: u64 = 1003;
const FOLDER_HEADER_TEXT_FILL_ID: u64 = 1004;
const FOLDER_HEADER_BADGE_ALIGN_ID: u64 = 1005;
const FOLDER_HEADER_BADGE_ID: u64 = 1006;
const SOURCE_DIVIDER_ALIGN_ID: u64 = 1010;
const SOURCE_DIVIDER_ID: u64 = 1011;

#[path = "sidebar_header/helpers.rs"]
mod helpers;

#[cfg(test)]
#[path = "sidebar_header/tests.rs"]
mod tests;

/// Slot-resolved recovery badge layout inside the folder-header surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RecoveryBadgeLayout {
    pub rect: Rect,
    pub label: String,
    pub active: bool,
}

/// Slot-resolved folder-header toggle layout inside the folder header.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FolderHeaderToggleLayout {
    pub rect: Rect,
    pub active: bool,
    pub enabled: bool,
}

/// Slot-resolved folder-header text rows and optional recovery badge.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SidebarFolderHeaderLayout {
    pub title_row: Rect,
    pub metadata_row: Option<Rect>,
    pub badge: Option<RecoveryBadgeLayout>,
    pub visibility_toggle_button: Option<FolderHeaderToggleLayout>,
    pub flatten_toggle_button: Option<FolderHeaderToggleLayout>,
}

/// Compute folder-header text rows and recovery badge through slotized helpers.
pub(crate) fn compute_sidebar_folder_header_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    recovery_in_progress: bool,
    recovery_entry_count: usize,
    show_all_items: bool,
    visibility_toggle_enabled: bool,
    flattened_view: bool,
    flattened_toggle_enabled: bool,
) -> SidebarFolderHeaderLayout {
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return SidebarFolderHeaderLayout {
            title_row: helpers::empty_rect(header_rect),
            metadata_row: None,
            badge: None,
            visibility_toggle_button: None,
            flatten_toggle_button: None,
        };
    }

    let (visibility_toggle_button, flatten_toggle_button) =
        helpers::compute_folder_header_toggle_layouts(
            header_rect,
            sizing,
            show_all_items,
            visibility_toggle_enabled,
            flattened_view,
            flattened_toggle_enabled,
        );
    let reserved_right_edge = visibility_toggle_button
        .as_ref()
        .map(|button| button.rect.min.x - sizing.sidebar_action_button_gap.max(2.0))
        .or_else(|| {
            flatten_toggle_button
                .as_ref()
                .map(|button| button.rect.min.x - sizing.sidebar_action_button_gap.max(2.0))
        })
        .unwrap_or(header_rect.max.x);
    let badge = helpers::compute_recovery_badge_layout(
        Rect::from_min_max(
            header_rect.min,
            Point::new(
                reserved_right_edge.max(header_rect.min.x),
                header_rect.max.y,
            ),
        ),
        sizing,
        recovery_in_progress,
        recovery_entry_count,
    );
    let text_start_x = header_rect.min.x + sizing.text_inset_x + sizing.header_label_gutter;
    let text_end_x = badge
        .as_ref()
        .map(|badge| badge.rect.min.x - sizing.text_inset_x)
        .unwrap_or(reserved_right_edge - sizing.text_inset_x);
    let text_bounds = Rect::from_min_max(
        Point::new(text_start_x, header_rect.min.y),
        Point::new(text_end_x.max(text_start_x), header_rect.max.y),
    );
    if text_bounds.width() <= 0.0 {
        return SidebarFolderHeaderLayout {
            title_row: helpers::empty_rect(header_rect),
            metadata_row: None,
            badge,
            visibility_toggle_button,
            flatten_toggle_button,
        };
    }

    let show_metadata = helpers::folder_header_has_metadata_row(header_rect, sizing);
    let column_children = helpers::build_text_rows(show_metadata, sizing);
    let text_tree = LayoutNode::container(
        FOLDER_HEADER_TEXT_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::PaddingBox,
            padding: Insets {
                top: sizing.text_inset_y.max(0.0),
                bottom: sizing.text_inset_y.max(0.0),
                ..Insets::default()
            },
            align_cross: CrossAlign::Stretch,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                FOLDER_HEADER_TEXT_COLUMN_ID,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: sizing.text_row_gap.max(0.0),
                    align_main: MainAlign::Start,
                    align_cross: CrossAlign::Stretch,
                    overflow: OverflowPolicy::Clip,
                    ..ContainerPolicy::default()
                },
                column_children,
            ),
        }],
    );
    let output = layout_tree(&text_tree, text_bounds);
    let title_row = output.rect_for_clamped(
        FOLDER_HEADER_TITLE_ID,
        helpers::empty_rect(text_bounds),
        text_bounds,
    );
    let metadata_row = if show_metadata {
        let row = output.rect_for_clamped(
            FOLDER_HEADER_META_ID,
            helpers::empty_rect(text_bounds),
            text_bounds,
        );
        (row.height() > 0.0).then_some(row)
    } else {
        None
    };
    SidebarFolderHeaderLayout {
        title_row,
        metadata_row,
        badge,
        visibility_toggle_button,
        flatten_toggle_button,
    }
}

/// Compute source/folder section divider geometry through slotized alignment.
pub(crate) fn compute_source_section_divider_rect(
    source_rows: Rect,
    folder_header: Rect,
    sizing: SizingTokens,
) -> Option<Rect> {
    if folder_header.height() <= 0.0 || source_rows.width() <= 0.0 {
        return None;
    }
    let divider_height = sizing.source_section_divider_width.max(0.5);
    let gap_top = source_rows.max.y;
    let gap_bottom = folder_header.min.y.max(gap_top);
    let fallback_bottom = (folder_header.min.y + divider_height).min(folder_header.max.y);
    let align_bounds = if gap_bottom - gap_top >= divider_height {
        Rect::from_min_max(
            Point::new(source_rows.min.x, gap_top),
            Point::new(source_rows.max.x, gap_bottom),
        )
    } else {
        Rect::from_min_max(
            Point::new(source_rows.min.x, folder_header.min.y),
            Point::new(source_rows.max.x, fallback_bottom),
        )
    };
    if align_bounds.height() <= 0.0 {
        return None;
    }
    let divider_tree = LayoutNode::container(
        SOURCE_DIVIDER_ALIGN_ID,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::Center,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(divider_height),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::new(0.0, f32::INFINITY, divider_height, divider_height),
                margin: Insets::default(),
                align_cross_override: Some(CrossAlign::Stretch),
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(
                SOURCE_DIVIDER_ID,
                Vector2::new(source_rows.width().max(1.0), divider_height.max(1.0)),
            ),
        }],
    );
    let output = layout_tree(&divider_tree, align_bounds);
    let rect = output.rect_for_clamped(
        SOURCE_DIVIDER_ID,
        helpers::empty_rect(align_bounds),
        Rect::from_min_max(
            Point::new(source_rows.min.x, source_rows.min.y),
            Point::new(source_rows.max.x, folder_header.max.y),
        ),
    );
    (rect.height() > 0.0).then_some(rect)
}
