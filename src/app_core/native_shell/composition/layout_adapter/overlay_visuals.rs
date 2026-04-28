//! Slotized visual geometry for prompt/progress/drag overlays.

use super::overlays::{
    ProgressOverlaySections, PromptOverlaySections, compute_drag_overlay_rect,
    compute_progress_overlay_sections, compute_prompt_overlay_sections,
};
use crate::gui::native_shell::style::SizingTokens;
use crate::gui::types::{Point, Rect};

/// Slot-resolved visual geometry for the confirmation prompt overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PromptOverlayVisualLayout {
    pub scrim: Rect,
    pub sections: PromptOverlaySections,
}

/// Slot-resolved visual geometry for the progress overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProgressOverlayVisualLayout {
    pub scrim: Option<Rect>,
    pub sections: ProgressOverlaySections,
    pub progress_fill: Option<Rect>,
}

/// Slot-resolved visual geometry for the drag overlay banner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragOverlayVisualLayout {
    pub banner: Rect,
}

/// Compute prompt overlay visual geometry used by rendering and hit-testing.
pub(crate) fn compute_prompt_overlay_visual_layout(
    root: Rect,
    content: Rect,
    sizing: SizingTokens,
    has_input: bool,
    has_target_label: bool,
) -> PromptOverlayVisualLayout {
    PromptOverlayVisualLayout {
        scrim: root,
        sections: compute_prompt_overlay_sections(content, sizing, has_input, has_target_label),
    }
}

/// Compute progress overlay visual geometry, including filled-track rect.
pub(crate) fn compute_progress_overlay_visual_layout(
    root: Rect,
    content: Rect,
    sizing: SizingTokens,
    modal: bool,
    progress_fraction: f32,
) -> ProgressOverlayVisualLayout {
    let sections = compute_progress_overlay_sections(content, sizing, modal);
    ProgressOverlayVisualLayout {
        scrim: modal.then_some(root),
        sections,
        progress_fill: compute_progress_fill_rect(sections.progress_bar, progress_fraction),
    }
}

/// Compute drag overlay visual geometry used by rendering and hit-testing.
pub(crate) fn compute_drag_overlay_visual_layout(
    content: Rect,
    status_bar: Rect,
    sizing: SizingTokens,
) -> DragOverlayVisualLayout {
    DragOverlayVisualLayout {
        banner: compute_drag_overlay_rect(content, status_bar, sizing),
    }
}

fn compute_progress_fill_rect(track: Rect, progress_fraction: f32) -> Option<Rect> {
    if track.width() <= 0.0 || track.height() <= 0.0 {
        return None;
    }
    let width = track.width() * progress_fraction.clamp(0.0, 1.0);
    if width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track.min,
        Point::new(track.min.x + width, track.max.y),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::style::StyleTokens;

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn prompt_visual_layout_uses_root_scrim_and_content_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let root = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 720.0));
        let content = Rect::from_min_max(Point::new(260.0, 60.0), Point::new(1240.0, 640.0));
        let visuals = compute_prompt_overlay_visual_layout(root, content, style.sizing, true, true);
        assert_eq!(visuals.scrim, root);
        assert_inside(content, visuals.sections.dialog);
        assert_inside(visuals.sections.dialog, visuals.sections.confirm_button);
        assert_inside(visuals.sections.dialog, visuals.sections.cancel_button);
    }

    #[test]
    fn progress_fill_rect_is_clamped_to_track_bounds() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let root = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(1280.0, 720.0));
        let content = Rect::from_min_max(Point::new(260.0, 60.0), Point::new(1240.0, 640.0));
        let visuals =
            compute_progress_overlay_visual_layout(root, content, style.sizing, true, 1.5);
        let fill = visuals.progress_fill.expect("filled progress rect");
        assert_eq!(visuals.scrim, Some(root));
        assert_inside(visuals.sections.progress_bar, fill);
        assert_eq!(fill.max.x, visuals.sections.progress_bar.max.x);
    }

    #[test]
    fn drag_banner_stays_inside_content_and_above_status() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let content = Rect::from_min_max(Point::new(260.0, 60.0), Point::new(1240.0, 640.0));
        let status = Rect::from_min_max(Point::new(20.0, 660.0), Point::new(1260.0, 700.0));
        let visuals = compute_drag_overlay_visual_layout(content, status, style.sizing);
        assert_inside(content, visuals.banner);
        assert!(visuals.banner.max.y <= status.min.y);
    }
}
