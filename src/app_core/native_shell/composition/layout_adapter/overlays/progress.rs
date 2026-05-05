//! Slotized progress overlay geometry.

use super::{ProgressOverlaySections, shared};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Rect, Vector2};

const PROGRESS_DIALOG_ROOT_ID: u64 = 930;
const PROGRESS_DIALOG_TOP_SPACER_ID: u64 = 931;
const PROGRESS_DIALOG_ID: u64 = 932;
const PROGRESS_DIALOG_BOTTOM_SPACER_ID: u64 = 933;
const PROGRESS_BAR_ROOT_ID: u64 = 934;
const PROGRESS_BAR_TOP_SPACER_ID: u64 = 935;
const PROGRESS_BAR_ID: u64 = 936;
const PROGRESS_CANCEL_ROW_ID: u64 = 937;
const PROGRESS_CANCEL_SPACER_ID: u64 = 938;
const PROGRESS_CANCEL_BUTTON_ID: u64 = 939;

/// Compute progress overlay dialog/progress bar/cancel sections inside content bounds.
pub(super) fn compute_progress_overlay_sections(
    content: Rect,
    sizing: SizingTokens,
    modal: bool,
) -> ProgressOverlaySections {
    let dialog = compute_progress_dialog_rect(content, sizing, modal);
    let progress_bar = compute_progress_bar_rect(dialog, sizing);
    let cancel_button = compute_progress_cancel_button_rect(dialog, sizing);
    ProgressOverlaySections {
        dialog,
        progress_bar,
        cancel_button,
    }
}

fn compute_progress_dialog_rect(content: Rect, sizing: SizingTokens, modal: bool) -> Rect {
    if content.width() <= 0.0 || content.height() <= 0.0 {
        return shared::empty_rect(content);
    }
    if modal {
        let width =
            (sizing.prompt_width * 0.85).min(content.width() - (sizing.overlay_padding * 2.0));
        let height = 96.0_f32.max(sizing.prompt_min_height * 0.72);
        let tree = LayoutNode::container(
            PROGRESS_DIALOG_ROOT_ID,
            ContainerPolicy {
                kind: ContainerKind::Column,
                align_main: MainAlign::Start,
                align_cross: CrossAlign::Stretch,
                overflow: OverflowPolicy::Clip,
                ..ContainerPolicy::default()
            },
            vec![
                shared::fill_child(PROGRESS_DIALOG_TOP_SPACER_ID, 0.28),
                shared::fixed_child(PROGRESS_DIALOG_ID, width, height, CrossAlign::Center),
                shared::fill_child(PROGRESS_DIALOG_BOTTOM_SPACER_ID, 0.72),
            ],
        );
        let output = layout_tree(&tree, content);
        return shared::rect_for(
            &output.rects,
            PROGRESS_DIALOG_ID,
            shared::empty_rect(content),
        );
    }

    let width = (sizing.prompt_width * 0.7).min(content.width() - (sizing.overlay_padding * 2.0));
    let height = 84.0;
    let bounds = shared::inset_uniform(content, sizing.overlay_padding.max(0.0));
    let tree = LayoutNode::container(
        PROGRESS_DIALOG_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::End,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![shared::fixed_child(
            PROGRESS_DIALOG_ID,
            width,
            height,
            CrossAlign::End,
        )],
    );
    let output = layout_tree(&tree, bounds);
    shared::rect_for(
        &output.rects,
        PROGRESS_DIALOG_ID,
        shared::empty_rect(content),
    )
}

fn compute_progress_bar_rect(dialog: Rect, sizing: SizingTokens) -> Rect {
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return shared::empty_rect(dialog);
    }
    let top =
        sizing.text_inset_y + sizing.font_header + sizing.font_meta + (sizing.text_row_gap * 2.0);
    let tree = LayoutNode::container(
        PROGRESS_BAR_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(top.max(0.0)),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(0.0, f32::INFINITY, top.max(0.0), top.max(0.0)),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                child: LayoutNode::widget(
                    PROGRESS_BAR_TOP_SPACER_ID,
                    Vector2::new(1.0, top.max(1.0)),
                ),
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(sizing.progress_bar_height.max(0.0)),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        0.0,
                        f32::INFINITY,
                        sizing.progress_bar_height.max(0.0),
                        sizing.progress_bar_height.max(0.0),
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                child: LayoutNode::widget(
                    PROGRESS_BAR_ID,
                    Vector2::new(1.0, sizing.progress_bar_height.max(1.0)),
                ),
            },
        ],
    );
    let output = layout_tree(
        &tree,
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    shared::rect_for(&output.rects, PROGRESS_BAR_ID, shared::empty_rect(dialog))
}

fn compute_progress_cancel_button_rect(dialog: Rect, sizing: SizingTokens) -> Rect {
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return shared::empty_rect(dialog);
    }
    let button_height = sizing.overlay_button_height.max(1.0);
    let tree = LayoutNode::container(
        PROGRESS_CANCEL_ROW_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(PROGRESS_CANCEL_SPACER_ID, Vector2::new(1.0, 1.0)),
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(button_height),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(0.0, f32::INFINITY, button_height, button_height),
                    margin: Insets {
                        bottom: sizing.text_inset_y.max(0.0),
                        ..Insets::default()
                    },
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: LayoutNode::container(
                    PROGRESS_CANCEL_ROW_ID + 1,
                    ContainerPolicy {
                        kind: ContainerKind::Row,
                        align_main: MainAlign::Start,
                        align_cross: CrossAlign::Stretch,
                        overflow: OverflowPolicy::Clip,
                        ..ContainerPolicy::default()
                    },
                    vec![
                        SlotChild {
                            slot: SlotParams::fill(),
                            child: LayoutNode::widget(
                                PROGRESS_CANCEL_ROW_ID + 2,
                                Vector2::new(1.0, 1.0),
                            ),
                        },
                        shared::fixed_width_button(
                            PROGRESS_CANCEL_BUTTON_ID,
                            sizing.overlay_button_width,
                            0.0,
                        ),
                    ],
                ),
            },
        ],
    );
    let output = layout_tree(
        &tree,
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    shared::rect_for(
        &output.rects,
        PROGRESS_CANCEL_BUTTON_ID,
        shared::empty_rect(dialog),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;
    use crate::gui::types::Point;

    #[test]
    fn non_modal_progress_overlay_stays_within_padded_content() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let content = Rect::from_min_max(Point::new(260.0, 60.0), Point::new(1240.0, 640.0));
        let sections = compute_progress_overlay_sections(content, style.sizing, false);
        assert!(sections.dialog.min.x >= content.min.x);
        assert!(sections.dialog.min.y >= content.min.y);
        assert!(sections.dialog.max.x <= content.max.x);
        assert!(sections.dialog.max.y <= content.max.y);
        assert!(sections.cancel_button.max.x <= sections.dialog.max.x);
    }
}
