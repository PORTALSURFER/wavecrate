//! Shared overlay geometry and shell-state drawing helpers.

use super::*;
use crate::gui::layout_core::stacked_row_rects;

/// Resolve the modal progress cancel-button hit target.
pub(in crate::app_core::native_shell::composition::state) fn progress_cancel_button(
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
pub(in crate::app_core::native_shell::composition::state) fn prompt_buttons(
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
pub(in crate::app_core::native_shell::composition::state) fn prompt_input_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> Option<Rect> {
    model.confirm_prompt.input_value.as_ref()?;
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
pub(in crate::app_core::native_shell::composition::state) fn drag_overlay_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> Rect {
    compute_drag_overlay_visual_layout(layout.content, layout.status_bar, style.sizing).banner
}

/// Build style tokens from the shell layout width and ui scale.
pub(in crate::app_core::native_shell::composition::state) fn style_for_layout(
    layout: &ShellLayout,
) -> StyleTokens {
    StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
}

/// Return whether the current prompt has a blocking validation error.
pub(in crate::app_core::native_shell::composition::state) fn prompt_has_validation_error(
    model: &AppModel,
) -> bool {
    model
        .confirm_prompt
        .input_error
        .as_ref()
        .is_some_and(|error| !error.trim().is_empty())
}

/// Draw a full border around one rect.
pub(in crate::app_core::native_shell::composition::state) fn push_border(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: crate::gui::types::Rgba8,
    stroke: f32,
) {
    push_border_sides(primitives, rect, color, stroke, BorderSides::ALL);
}

/// Per-edge border ownership used to avoid double-width seams between touching panels.
pub(in crate::app_core::native_shell::composition::state) type BorderSides =
    crate::gui::paint::BorderSides;

/// Draw only the requested border edges for one rect.
pub(in crate::app_core::native_shell::composition::state) fn push_border_sides(
    primitives: &mut impl PrimitiveSink,
    rect: Rect,
    color: crate::gui::types::Rgba8,
    stroke: f32,
    sides: BorderSides,
) {
    for fill in crate::gui::paint::border_fill_rects(rect, color, stroke, sides) {
        emit_primitive(primitives, Primitive::Rect(fill));
    }
}

/// Build vertically stacked row rects inside one column.
pub(in crate::app_core::native_shell::composition::state) fn build_stacked_rows(
    column: Rect,
    rows: usize,
    gap: f32,
    row_height: f32,
) -> Vec<Rect> {
    stacked_row_rects(column, rows, gap, row_height)
}
