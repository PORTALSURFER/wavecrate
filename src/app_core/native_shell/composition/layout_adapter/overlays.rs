//! Slotized helpers for prompt/progress/drag overlay geometry.

#[path = "overlays/drag.rs"]
mod drag;
#[path = "overlays/progress.rs"]
mod progress;
#[path = "overlays/prompt.rs"]
mod prompt;
#[path = "overlays/shared.rs"]
mod shared;
#[path = "overlays/text.rs"]
mod text;

use super::super::style::SizingTokens;
use crate::gui::types::Rect;

/// Slot-resolved prompt overlay geometry for hit-testing and rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PromptOverlaySections {
    pub dialog: Rect,
    pub confirm_button: Rect,
    pub cancel_button: Rect,
    pub input: Option<Rect>,
}

/// Slot-resolved progress overlay geometry for hit-testing and rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProgressOverlaySections {
    pub dialog: Rect,
    pub progress_bar: Rect,
    pub cancel_button: Rect,
}

/// Slot-resolved prompt overlay text-line geometry for rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PromptOverlayTextLayout {
    pub title: Rect,
    pub message: Rect,
    pub target: Option<Rect>,
    pub input_text: Option<Rect>,
    pub input_error: Option<Rect>,
    pub confirm_label: Rect,
    pub cancel_label: Rect,
}

/// Slot-resolved progress overlay text-line geometry for rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProgressOverlayTextLayout {
    pub title: Rect,
    pub detail: Option<Rect>,
    pub counter: Rect,
    pub cancel_label: Rect,
}

/// Slot-resolved drag overlay text-line geometry for rendering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragOverlayTextLayout {
    pub label: Rect,
}

/// Compute prompt overlay dialog/button/input sections inside content bounds.
pub(crate) fn compute_prompt_overlay_sections(
    content: Rect,
    sizing: SizingTokens,
    has_input: bool,
    has_target_label: bool,
) -> PromptOverlaySections {
    prompt::compute_prompt_overlay_sections(content, sizing, has_input, has_target_label)
}

/// Compute progress overlay dialog/progress bar/cancel sections inside content bounds.
pub(crate) fn compute_progress_overlay_sections(
    content: Rect,
    sizing: SizingTokens,
    modal: bool,
) -> ProgressOverlaySections {
    progress::compute_progress_overlay_sections(content, sizing, modal)
}

/// Compute drag overlay banner rect between content and status bars.
pub(crate) fn compute_drag_overlay_rect(
    content: Rect,
    status_bar: Rect,
    sizing: SizingTokens,
) -> Rect {
    drag::compute_drag_overlay_rect(content, status_bar, sizing)
}

/// Compute prompt overlay text-line sections for dialog, input, and action buttons.
pub(crate) fn compute_prompt_overlay_text_layout(
    sections: PromptOverlaySections,
    sizing: SizingTokens,
    has_target_label: bool,
    has_input_error: bool,
) -> PromptOverlayTextLayout {
    text::compute_prompt_overlay_text_layout(sections, sizing, has_target_label, has_input_error)
}

/// Compute progress overlay text-line sections for title/detail/counter/cancel copy.
pub(crate) fn compute_progress_overlay_text_layout(
    sections: ProgressOverlaySections,
    sizing: SizingTokens,
    has_detail: bool,
    has_cancel_button: bool,
) -> ProgressOverlayTextLayout {
    text::compute_progress_overlay_text_layout(sections, sizing, has_detail, has_cancel_button)
}

/// Compute drag overlay label text-line bounds.
pub(crate) fn compute_drag_overlay_text_layout(
    banner: Rect,
    sizing: SizingTokens,
) -> DragOverlayTextLayout {
    text::compute_drag_overlay_text_layout(banner, sizing)
}
