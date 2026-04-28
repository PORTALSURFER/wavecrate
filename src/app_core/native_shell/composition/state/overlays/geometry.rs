//! Shared overlay geometry and shell-state drawing helpers.

use super::*;

/// Resolve the modal progress cancel-button hit target.
pub(in crate::gui::native_shell::state) fn progress_cancel_button(
    layout: &ShellLayout,
    style: &StyleTokens,
    modal: bool,
) -> Rect {
    compute_progress_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        modal,
        0.0,
    )
    .sections
    .cancel_button
}

/// Resolve the prompt confirm/cancel button hit targets.
pub(in crate::gui::native_shell::state) fn prompt_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> (Rect, Rect) {
    let sections = compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        false,
        false,
    )
    .sections;
    (sections.confirm_button, sections.cancel_button)
}

/// Resolve the prompt input hit target when the current prompt owns a text field.
pub(in crate::gui::native_shell::state) fn prompt_input_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<Rect> {
    if model.confirm_prompt.input_value.is_none() {
        return None;
    }
    compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        true,
        model.confirm_prompt.target_label.is_some(),
    )
    .sections
    .input
}

/// Resolve the drag overlay banner rect.
pub(in crate::gui::native_shell::state) fn drag_overlay_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> Rect {
    compute_drag_overlay_visual_layout(layout.content, layout.status_bar, style.sizing).banner
}

/// Build style tokens from the shell layout width and ui scale.
pub(in crate::gui::native_shell::state) fn style_for_layout(layout: &ShellLayout) -> StyleTokens {
    StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
}

/// Return whether the current prompt has a blocking validation error.
pub(in crate::gui::native_shell::state) fn prompt_has_validation_error(model: &AppModel) -> bool {
    model
        .confirm_prompt
        .input_error
        .as_ref()
        .is_some_and(|error| !error.trim().is_empty())
}

/// Draw a full border around one rect.
pub(in crate::gui::native_shell::state) fn push_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: crate::gui::types::Rgba8,
    stroke: f32,
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
            color,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
            color,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
            color,
        }),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
            color,
        }),
    );
}

/// Per-edge border ownership used to avoid double-width seams between touching panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui::native_shell::state) struct BorderSides {
    pub(in crate::gui::native_shell::state) top: bool,
    pub(in crate::gui::native_shell::state) bottom: bool,
    pub(in crate::gui::native_shell::state) left: bool,
    pub(in crate::gui::native_shell::state) right: bool,
}

impl BorderSides {
    /// Draw all four edges.
    pub(in crate::gui::native_shell::state) const ALL: Self = Self {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
}

/// Draw only the requested border edges for one rect.
pub(in crate::gui::native_shell::state) fn push_border_sides(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: crate::gui::types::Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    let stroke = stroke.max(1.0);
    if rect.width() <= stroke * 2.0 || rect.height() <= stroke * 2.0 {
        return;
    }
    if sides.top {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(rect.min, Point::new(rect.max.x, rect.min.y + stroke)),
                color,
            }),
        );
    }
    if sides.bottom {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.min.x, rect.max.y - stroke), rect.max),
                color,
            }),
        );
    }
    if sides.left {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(rect.min, Point::new(rect.min.x + stroke, rect.max.y)),
                color,
            }),
        );
    }
    if sides.right {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: Rect::from_min_max(Point::new(rect.max.x - stroke, rect.min.y), rect.max),
                color,
            }),
        );
    }
}

/// Build vertically stacked row rects inside one column.
pub(in crate::gui::native_shell::state) fn build_stacked_rows(
    column: Rect,
    rows: usize,
    gap: f32,
    row_height: f32,
) -> Vec<Rect> {
    if rows == 0 {
        return Vec::new();
    }
    let row_height = row_height.max(8.0).round().max(1.0);
    let gap = gap.max(0.0);
    let stride = row_height + gap;
    let column_min_y = column.min.y.round();
    let column_max_y = column.max.y.round().max(column_min_y);
    let mut output = Vec::with_capacity(rows);
    for index in 0..rows {
        let y = (column_min_y + (index as f32 * stride)).round();
        let max_y = (y + row_height).min(column_max_y);
        if max_y <= y {
            break;
        }
        output.push(Rect::from_min_max(
            Point::new(column.min.x, y),
            Point::new(column.max.x, max_y),
        ));
        if max_y >= column_max_y {
            break;
        }
    }
    output
}
