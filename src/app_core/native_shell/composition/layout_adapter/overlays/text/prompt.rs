use super::super::{PromptOverlaySections, PromptOverlayTextLayout, shared};
use super::common::{centered_line_in_rect, column_tree, fixed_height_child, top_line_in_rect};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Point, Rect};

const PROMPT_TEXT_ROOT_ID: u64 = 980;
const PROMPT_TEXT_TITLE_ID: u64 = 981;
const PROMPT_TEXT_MESSAGE_ID: u64 = 982;
const PROMPT_TEXT_TARGET_ID: u64 = 983;
const PROMPT_TEXT_INPUT_ID: u64 = 984;
const PROMPT_TEXT_INPUT_ERROR_ID: u64 = 985;
const PROMPT_TEXT_CONFIRM_ID: u64 = 986;
const PROMPT_TEXT_CANCEL_ID: u64 = 987;

struct PromptDialogRows {
    title: Rect,
    message: Rect,
    target: Option<Rect>,
}

/// Compute prompt overlay text-line sections for dialog, input, and action buttons.
pub(crate) fn compute_prompt_overlay_text_layout(
    sections: PromptOverlaySections,
    sizing: SizingTokens,
    has_target_label: bool,
    has_input_error: bool,
) -> PromptOverlayTextLayout {
    let rows = compute_prompt_dialog_rows(sections.dialog, sizing, has_target_label);
    let input_text = sections
        .input
        .map(|input| top_line_in_rect(input, sizing, sizing.font_meta, PROMPT_TEXT_INPUT_ID));
    let input_error = compute_prompt_input_error_line(
        sections,
        sizing,
        has_input_error,
        PROMPT_TEXT_INPUT_ERROR_ID,
    );
    PromptOverlayTextLayout {
        title: rows.title,
        message: rows.message,
        target: rows.target,
        input_text,
        input_error,
        confirm_label: centered_line_in_rect(
            sections.confirm_button,
            sizing,
            sizing.font_meta,
            PROMPT_TEXT_CONFIRM_ID,
        ),
        cancel_label: centered_line_in_rect(
            sections.cancel_button,
            sizing,
            sizing.font_meta,
            PROMPT_TEXT_CANCEL_ID,
        ),
    }
}

fn compute_prompt_dialog_rows(
    dialog: Rect,
    sizing: SizingTokens,
    has_target: bool,
) -> PromptDialogRows {
    let empty = shared::empty_rect(dialog);
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return PromptDialogRows {
            title: empty,
            message: empty,
            target: if has_target { Some(empty) } else { None },
        };
    }
    let mut children = vec![
        fixed_height_child(PROMPT_TEXT_ROOT_ID + 10, sizing.text_inset_y.max(0.0)),
        fixed_height_child(PROMPT_TEXT_TITLE_ID, sizing.font_title.max(1.0)),
        fixed_height_child(PROMPT_TEXT_ROOT_ID + 11, sizing.text_row_gap.max(0.0)),
        fixed_height_child(PROMPT_TEXT_MESSAGE_ID, sizing.font_meta.max(1.0)),
    ];
    if has_target {
        children.push(fixed_height_child(
            PROMPT_TEXT_ROOT_ID + 12,
            sizing.text_row_gap.max(0.0),
        ));
        children.push(fixed_height_child(
            PROMPT_TEXT_TARGET_ID,
            sizing.font_meta.max(1.0),
        ));
    }
    let output = crate::gui::layout_core::layout_tree(
        &column_tree(PROMPT_TEXT_ROOT_ID, children),
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    PromptDialogRows {
        title: shared::rect_for(&output.rects, PROMPT_TEXT_TITLE_ID, empty),
        message: shared::rect_for(&output.rects, PROMPT_TEXT_MESSAGE_ID, empty),
        target: has_target.then_some(shared::rect_for(
            &output.rects,
            PROMPT_TEXT_TARGET_ID,
            empty,
        )),
    }
}

fn compute_prompt_input_error_line(
    sections: PromptOverlaySections,
    sizing: SizingTokens,
    has_input_error: bool,
    node_id: u64,
) -> Option<Rect> {
    if !has_input_error {
        return None;
    }
    let input = sections.input?;
    let top = input.max.y + sizing.text_row_gap.max(0.0);
    let bottom = sections
        .confirm_button
        .min
        .y
        .min(sections.dialog.max.y - sizing.text_inset_y.max(0.0));
    if bottom <= top {
        return Some(shared::empty_rect(input));
    }
    let bounds = Rect::from_min_max(
        Point::new(input.min.x, top),
        Point::new(input.max.x.max(input.min.x), bottom),
    );
    Some(top_line_in_rect(bounds, sizing, sizing.font_meta, node_id))
}
