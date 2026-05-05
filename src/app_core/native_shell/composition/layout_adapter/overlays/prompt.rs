//! Slotized prompt overlay geometry.

use super::{PromptOverlaySections, shared};
use crate::gui::layout_core::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutNode, MainAlign,
    OverflowPolicy, SizeModeCross, SizeModeMain, SlotChild, SlotParams, layout_tree,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Rect, Vector2};

const PROMPT_DIALOG_ROOT_ID: u64 = 900;
const PROMPT_DIALOG_TOP_SPACER_ID: u64 = 901;
const PROMPT_DIALOG_ID: u64 = 902;
const PROMPT_DIALOG_BOTTOM_SPACER_ID: u64 = 903;
const PROMPT_BUTTON_ROW_ID: u64 = 904;
const PROMPT_BUTTON_ROW_SPACER_ID: u64 = 905;
const PROMPT_CONFIRM_BUTTON_ID: u64 = 906;
const PROMPT_CANCEL_BUTTON_ID: u64 = 907;
const PROMPT_INPUT_ROOT_ID: u64 = 908;
const PROMPT_INPUT_TOP_SPACER_ID: u64 = 909;
const PROMPT_INPUT_ID: u64 = 910;

/// Compute prompt overlay dialog/button/input sections inside content bounds.
pub(super) fn compute_prompt_overlay_sections(
    content: Rect,
    sizing: SizingTokens,
    has_input: bool,
    has_target_label: bool,
) -> PromptOverlaySections {
    let dialog = compute_prompt_dialog_rect(content, sizing);
    let (confirm_button, cancel_button) = compute_prompt_buttons(dialog, sizing);
    let input = if has_input {
        compute_prompt_input_rect(dialog, sizing, has_target_label)
    } else {
        None
    };
    PromptOverlaySections {
        dialog,
        confirm_button,
        cancel_button,
        input,
    }
}

fn compute_prompt_dialog_rect(content: Rect, sizing: SizingTokens) -> Rect {
    if content.width() <= 0.0 || content.height() <= 0.0 {
        return shared::empty_rect(content);
    }
    let width = sizing
        .prompt_width
        .min(content.width() - (sizing.overlay_padding * 2.0))
        .max(260.0);
    let height = sizing
        .prompt_min_height
        .min(content.height() - (sizing.overlay_padding * 2.0))
        .max(108.0);
    let tree = LayoutNode::container(
        PROMPT_DIALOG_ROOT_ID,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_main: MainAlign::Start,
            align_cross: CrossAlign::Stretch,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            shared::fill_child(PROMPT_DIALOG_TOP_SPACER_ID, 0.35),
            shared::fixed_child(PROMPT_DIALOG_ID, width, height, CrossAlign::Center),
            shared::fill_child(PROMPT_DIALOG_BOTTOM_SPACER_ID, 0.65),
        ],
    );
    let output = layout_tree(&tree, content);
    shared::rect_for(&output.rects, PROMPT_DIALOG_ID, shared::empty_rect(content))
}

fn compute_prompt_buttons(dialog: Rect, sizing: SizingTokens) -> (Rect, Rect) {
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        let empty = shared::empty_rect(dialog);
        return (empty, empty);
    }
    let button_height = sizing.overlay_button_height.max(1.0);
    let tree = LayoutNode::container(
        PROMPT_BUTTON_ROW_ID,
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
                child: LayoutNode::widget(PROMPT_BUTTON_ROW_SPACER_ID, Vector2::new(1.0, 1.0)),
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
                    PROMPT_BUTTON_ROW_SPACER_ID + 1,
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
                                PROMPT_BUTTON_ROW_SPACER_ID + 2,
                                Vector2::new(1.0, 1.0),
                            ),
                        },
                        shared::fixed_width_button(
                            PROMPT_CONFIRM_BUTTON_ID,
                            sizing.overlay_button_width,
                            0.0,
                        ),
                        shared::fixed_width_button(
                            PROMPT_CANCEL_BUTTON_ID,
                            sizing.overlay_button_width,
                            sizing.action_button_gap.max(0.0),
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
    (
        shared::rect_for(
            &output.rects,
            PROMPT_CONFIRM_BUTTON_ID,
            shared::empty_rect(dialog),
        ),
        shared::rect_for(
            &output.rects,
            PROMPT_CANCEL_BUTTON_ID,
            shared::empty_rect(dialog),
        ),
    )
}

fn compute_prompt_input_rect(
    dialog: Rect,
    sizing: SizingTokens,
    has_target_label: bool,
) -> Option<Rect> {
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return None;
    }
    let input_top = sizing.text_inset_y
        + sizing.font_title
        + sizing.font_meta
        + (sizing.text_row_gap * 4.0)
        + if has_target_label {
            sizing.font_meta + sizing.text_row_gap
        } else {
            0.0
        };
    let input_height = (sizing.overlay_button_height - 2.0).max(18.0);
    let max_top =
        (dialog.height() - sizing.overlay_button_height - sizing.text_inset_y - 6.0).max(0.0);
    let top = input_top.min(max_top);
    let tree = LayoutNode::container(
        PROMPT_INPUT_ROOT_ID,
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
                    PROMPT_INPUT_TOP_SPACER_ID,
                    Vector2::new(1.0, top.max(1.0)),
                ),
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(input_height),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(
                        0.0,
                        f32::INFINITY,
                        input_height.max(0.0),
                        input_height.max(0.0),
                    ),
                    margin: Insets::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                child: LayoutNode::widget(
                    PROMPT_INPUT_ID,
                    Vector2::new(1.0, input_height.max(1.0)),
                ),
            },
        ],
    );
    let output = layout_tree(
        &tree,
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    Some(shared::rect_for(
        &output.rects,
        PROMPT_INPUT_ID,
        shared::empty_rect(dialog),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;
    use crate::gui::types::Point;

    #[test]
    fn prompt_sections_keep_buttons_inside_dialog() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let content = Rect::from_min_max(Point::new(260.0, 60.0), Point::new(1240.0, 640.0));
        let sections = compute_prompt_overlay_sections(content, style.sizing, true, false);
        assert!(sections.dialog.width() > 0.0);
        assert!(sections.confirm_button.min.x >= sections.dialog.min.x);
        assert!(sections.confirm_button.max.x <= sections.dialog.max.x);
        assert!(sections.cancel_button.min.x >= sections.dialog.min.x);
        assert!(sections.cancel_button.max.x <= sections.dialog.max.x);
        assert!(sections.input.is_some());
    }
}
