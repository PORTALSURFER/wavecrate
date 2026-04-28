use super::super::DragOverlayTextLayout;
use super::common::centered_line_in_rect;
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::Rect;

const DRAG_TEXT_LABEL_ID: u64 = 995;

/// Compute drag overlay label text-line bounds.
pub(crate) fn compute_drag_overlay_text_layout(
    banner: Rect,
    sizing: SizingTokens,
) -> DragOverlayTextLayout {
    DragOverlayTextLayout {
        label: centered_line_in_rect(banner, sizing, sizing.font_meta, DRAG_TEXT_LABEL_ID),
    }
}
