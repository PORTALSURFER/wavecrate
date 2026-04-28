use super::super::super::style::SizingTokens;
use super::{
    FOLDER_HEADER_BADGE_ALIGN_ID, FOLDER_HEADER_BADGE_ID, FOLDER_HEADER_META_ID,
    FOLDER_HEADER_TEXT_FILL_ID, FOLDER_HEADER_TITLE_ID, FolderHeaderToggleLayout,
    RecoveryBadgeLayout,
};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::types::{Point, Rect, Vector2};

pub(crate) fn compute_recovery_badge_layout(
    header_rect: Rect,
    sizing: SizingTokens,
    recovery_in_progress: bool,
    recovery_entry_count: usize,
) -> Option<RecoveryBadgeLayout> {
    if !recovery_in_progress && recovery_entry_count == 0 {
        return None;
    }
    let available_width = (header_rect.width() - (sizing.text_inset_x * 2.0)).max(0.0);
    if available_width < 12.0 {
        return None;
    }
    let label = compact_recovery_badge_label(
        recovery_in_progress,
        recovery_entry_count,
        available_width,
        sizing,
    )?;
    let approx_char_width = (sizing.font_meta * 0.56).max(1.0);
    let label_width = label.chars().count() as f32 * approx_char_width;
    let badge_width = (label_width + (sizing.recovery_badge_padding_x * 2.0))
        .max(sizing.recovery_badge_min_width.min(available_width))
        .min(available_width);
    let badge_height = sizing
        .recovery_badge_height
        .min((header_rect.height() - 2.0).max(10.0));
    let badge_bounds = inset_horizontal(header_rect, sizing.text_inset_x.max(0.0));
    let badge_tree = LayoutNode::container(
        FOLDER_HEADER_BADGE_ALIGN_ID,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::Center,
            align_cross: CrossAlign::End,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(badge_height),
                size_cross: SizeModeCross::Fixed(badge_width),
                constraints: Constraints::new(badge_width, badge_width, badge_height, badge_height),
                margin: Insets::default(),
                align_cross_override: Some(CrossAlign::End),
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(
                FOLDER_HEADER_BADGE_ID,
                Vector2::new(badge_width.max(1.0), badge_height.max(1.0)),
            ),
        }],
    );
    let output = layout_tree(&badge_tree, badge_bounds);
    let rect = clamp_rect_to_bounds(
        rect_for(
            &output.rects,
            FOLDER_HEADER_BADGE_ID,
            empty_rect(badge_bounds),
        ),
        header_rect,
    );
    (rect.height() > 0.0).then_some(RecoveryBadgeLayout {
        rect,
        label,
        active: recovery_in_progress,
    })
}

pub(crate) fn compute_folder_header_toggle_layouts(
    header_rect: Rect,
    sizing: SizingTokens,
    show_all_folders: bool,
    visibility_enabled: bool,
    flattened_view: bool,
    flattened_enabled: bool,
) -> (
    Option<FolderHeaderToggleLayout>,
    Option<FolderHeaderToggleLayout>,
) {
    let available_width = (header_rect.width() - (sizing.text_inset_x * 2.0)).max(0.0);
    let button_size = sizing
        .sidebar_action_button_height
        .min((header_rect.height() - 2.0).max(10.0))
        .max(10.0);
    if available_width < button_size {
        return (None, None);
    }
    let max_x = header_rect.max.x - sizing.text_inset_x.max(0.0);
    let min_y = header_rect.min.y + ((header_rect.height() - button_size) * 0.5).floor();
    let gap = sizing.sidebar_action_button_gap.max(2.0);
    let two_button_width = (button_size * 2.0) + gap;
    let min_inner_x = header_rect.min.x + sizing.text_inset_x.max(0.0);

    let flatten_toggle_button = if available_width >= two_button_width {
        let flatten_min_x = (max_x - button_size).max(min_inner_x);
        let rect = Rect::from_min_max(
            Point::new(flatten_min_x, min_y),
            Point::new(max_x, (min_y + button_size).min(header_rect.max.y)),
        );
        Some(FolderHeaderToggleLayout {
            rect,
            active: flattened_view,
            enabled: flattened_enabled,
        })
    } else {
        None
    };

    let visibility_max_x = flatten_toggle_button
        .as_ref()
        .map(|button| button.rect.min.x - gap)
        .unwrap_or(max_x);
    let visibility_min_x = (visibility_max_x - button_size).max(min_inner_x);
    let visibility_rect = Rect::from_min_max(
        Point::new(visibility_min_x, min_y),
        Point::new(
            visibility_max_x,
            (min_y + button_size).min(header_rect.max.y),
        ),
    );
    let visibility_toggle_button = (visibility_rect.width() > 0.0
        && visibility_rect.height() > 0.0)
        .then_some(FolderHeaderToggleLayout {
            rect: visibility_rect,
            active: show_all_folders,
            enabled: visibility_enabled,
        });

    (visibility_toggle_button, flatten_toggle_button)
}

pub(crate) fn folder_header_has_metadata_row(header_rect: Rect, sizing: SizingTokens) -> bool {
    let required_height =
        (sizing.text_inset_y * 2.0) + sizing.font_header + sizing.text_row_gap + sizing.font_meta;
    header_rect.height() >= required_height
}

pub(crate) fn build_text_rows(show_metadata: bool, sizing: SizingTokens) -> Vec<SlotChild> {
    let mut rows = Vec::with_capacity(if show_metadata { 3 } else { 2 });
    rows.push(fixed_height_child(
        FOLDER_HEADER_TITLE_ID,
        sizing.font_header.max(1.0),
    ));
    if show_metadata {
        rows.push(fixed_height_child(
            FOLDER_HEADER_META_ID,
            sizing.font_meta.max(1.0),
        ));
    }
    rows.push(SlotChild {
        slot: SlotParams::fill(),
        child: LayoutNode::widget(FOLDER_HEADER_TEXT_FILL_ID, Vector2::new(1.0, 1.0)),
    });
    rows
}

pub(crate) fn clamp_rect_to_bounds(rect: Rect, bounds: Rect) -> Rect {
    let min = Point::new(rect.min.x.max(bounds.min.x), rect.min.y.max(bounds.min.y));
    let max = Point::new(rect.max.x.min(bounds.max.x), rect.max.y.min(bounds.max.y));
    if max.x < min.x || max.y < min.y {
        return Rect::from_min_max(bounds.min, bounds.min);
    }
    Rect::from_min_max(min, max)
}

pub(crate) fn rect_for(
    rects: &std::collections::BTreeMap<u64, Rect>,
    id: u64,
    fallback: Rect,
) -> Rect {
    rects.get(&id).copied().unwrap_or(fallback)
}

pub(crate) fn empty_rect(bounds: Rect) -> Rect {
    Rect::from_min_max(bounds.min, bounds.min)
}

fn compact_recovery_badge_label(
    recovery_in_progress: bool,
    recovery_entry_count: usize,
    available_width: f32,
    sizing: SizingTokens,
) -> Option<String> {
    let approx_char_width = (sizing.font_meta * 0.56).max(1.0);
    let wide_label_fits = |label: &str| {
        (label.chars().count() as f32 * approx_char_width) + (sizing.recovery_badge_padding_x * 2.0)
            <= available_width
    };
    if recovery_in_progress {
        return ["Recovery", "Active", "R"]
            .iter()
            .find(|label| wide_label_fits(label))
            .map(|label| (*label).to_string());
    }
    let long_label = format!("{recovery_entry_count} entries");
    if wide_label_fits(&long_label) {
        return Some(long_label);
    }
    Some(recovery_entry_count.to_string())
}

fn inset_horizontal(rect: Rect, inset: f32) -> Rect {
    let inset = inset.max(0.0).min((rect.width() * 0.5).max(0.0));
    Rect::from_min_max(
        Point::new(rect.min.x + inset, rect.min.y),
        Point::new(rect.max.x - inset, rect.max.y),
    )
}

fn fixed_height_child(node_id: u64, height: f32) -> SlotChild {
    SlotChild {
        slot: SlotParams {
            size_main: SizeModeMain::Fixed(height),
            size_cross: SizeModeCross::Fill,
            constraints: Constraints::new(0.0, f32::INFINITY, height, height),
            margin: Insets::default(),
            align_cross_override: Some(CrossAlign::Stretch),
            allow_fixed_compress: false,
        },
        child: LayoutNode::widget(node_id, Vector2::new(1.0, height.max(1.0))),
    }
}
