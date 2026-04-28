use super::super::{ProgressOverlaySections, ProgressOverlayTextLayout, shared};
use super::common::{centered_line_in_rect, column_tree, fixed_height_child, top_line_in_bounds};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Point, Rect};

const PROGRESS_TEXT_ROOT_ID: u64 = 990;
const PROGRESS_TEXT_TITLE_ID: u64 = 991;
const PROGRESS_TEXT_DETAIL_ID: u64 = 992;
const PROGRESS_TEXT_COUNTER_ID: u64 = 993;
const PROGRESS_TEXT_CANCEL_ID: u64 = 994;

struct ProgressDialogRows {
    title: Rect,
    detail: Option<Rect>,
}

/// Compute progress overlay text-line sections for title/detail/counter/cancel copy.
pub(crate) fn compute_progress_overlay_text_layout(
    sections: ProgressOverlaySections,
    sizing: SizingTokens,
    has_detail: bool,
    has_cancel_button: bool,
) -> ProgressOverlayTextLayout {
    let rows = compute_progress_dialog_rows(sections.dialog, sizing, has_detail);
    ProgressOverlayTextLayout {
        title: rows.title,
        detail: rows.detail,
        counter: compute_progress_counter_line(
            sections.dialog,
            sections.progress_bar,
            if has_cancel_button {
                Some(sections.cancel_button)
            } else {
                None
            },
            sizing,
            PROGRESS_TEXT_COUNTER_ID,
        ),
        cancel_label: if has_cancel_button {
            centered_line_in_rect(
                sections.cancel_button,
                sizing,
                sizing.font_meta,
                PROGRESS_TEXT_CANCEL_ID,
            )
        } else {
            shared::empty_rect(sections.cancel_button)
        },
    }
}

fn compute_progress_dialog_rows(
    dialog: Rect,
    sizing: SizingTokens,
    has_detail: bool,
) -> ProgressDialogRows {
    let empty = shared::empty_rect(dialog);
    if dialog.width() <= 0.0 || dialog.height() <= 0.0 {
        return ProgressDialogRows {
            title: empty,
            detail: if has_detail { Some(empty) } else { None },
        };
    }
    let mut children = vec![
        fixed_height_child(PROGRESS_TEXT_ROOT_ID + 10, sizing.text_inset_y.max(0.0)),
        fixed_height_child(PROGRESS_TEXT_TITLE_ID, sizing.font_header.max(1.0)),
    ];
    if has_detail {
        children.push(fixed_height_child(
            PROGRESS_TEXT_ROOT_ID + 11,
            sizing.text_row_gap.max(0.0),
        ));
        children.push(fixed_height_child(
            PROGRESS_TEXT_DETAIL_ID,
            sizing.font_meta.max(1.0),
        ));
    }
    let output = crate::gui::layout_core::layout_tree(
        &column_tree(PROGRESS_TEXT_ROOT_ID, children),
        shared::inset_horizontal(dialog, sizing.text_inset_x.max(0.0)),
    );
    ProgressDialogRows {
        title: shared::rect_for(&output.rects, PROGRESS_TEXT_TITLE_ID, empty),
        detail: has_detail.then_some(shared::rect_for(
            &output.rects,
            PROGRESS_TEXT_DETAIL_ID,
            empty,
        )),
    }
}

fn compute_progress_counter_line(
    dialog: Rect,
    progress_bar: Rect,
    cancel_button: Option<Rect>,
    sizing: SizingTokens,
    node_id: u64,
) -> Rect {
    let top = progress_bar.max.y + sizing.text_row_gap.max(0.0);
    let bottom = cancel_button
        .map(|button| button.min.y - sizing.text_row_gap.max(0.0))
        .unwrap_or(dialog.max.y - sizing.text_inset_y.max(0.0));
    if top >= bottom || progress_bar.width() <= 0.0 {
        return shared::empty_rect(progress_bar);
    }
    let bounds = Rect::from_min_max(
        Point::new(progress_bar.min.x, top),
        Point::new(progress_bar.max.x, bottom),
    );
    top_line_in_bounds(bounds, sizing.font_meta, node_id)
}
