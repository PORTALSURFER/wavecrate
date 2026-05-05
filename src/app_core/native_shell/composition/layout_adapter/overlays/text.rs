//! Slotized text-line geometry for prompt/progress/drag overlays.

#[path = "text/common.rs"]
mod common;
#[path = "text/drag.rs"]
mod drag;
#[path = "text/progress.rs"]
mod progress;
#[path = "text/prompt.rs"]
mod prompt;

pub(super) use drag::compute_drag_overlay_text_layout;
pub(super) use progress::compute_progress_overlay_text_layout;
pub(super) use prompt::compute_prompt_overlay_text_layout;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::native_shell::layout_adapter::overlays::{
        ProgressOverlaySections, PromptOverlaySections,
    };
    use crate::gui::native_shell::style::StyleTokens;
    use crate::gui::types::{Point, Rect};

    fn assert_inside(outer: Rect, inner: Rect) {
        assert!(inner.min.x >= outer.min.x);
        assert!(inner.min.y >= outer.min.y);
        assert!(inner.max.x <= outer.max.x);
        assert!(inner.max.y <= outer.max.y);
    }

    #[test]
    fn prompt_text_layout_stays_inside_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let sections = PromptOverlaySections {
            dialog: Rect::from_min_max(Point::new(400.0, 180.0), Point::new(920.0, 460.0)),
            confirm_button: Rect::from_min_max(Point::new(740.0, 418.0), Point::new(824.0, 438.0)),
            cancel_button: Rect::from_min_max(Point::new(836.0, 418.0), Point::new(920.0, 438.0)),
            input: Some(Rect::from_min_max(
                Point::new(420.0, 320.0),
                Point::new(900.0, 342.0),
            )),
        };
        let layout = compute_prompt_overlay_text_layout(sections, style.sizing, true, true);
        assert_inside(sections.dialog, layout.title);
        assert_inside(sections.dialog, layout.message);
        assert_inside(sections.dialog, layout.target.expect("target row"));
        assert_inside(
            sections.input.expect("input"),
            layout.input_text.expect("input text"),
        );
        assert_inside(sections.confirm_button, layout.confirm_label);
        assert_inside(sections.cancel_button, layout.cancel_label);
        assert!(
            layout.input_error.expect("input error").min.y >= sections.input.expect("input").max.y
        );
    }

    #[test]
    fn progress_text_layout_stays_inside_sections() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let sections = ProgressOverlaySections {
            dialog: Rect::from_min_max(Point::new(560.0, 110.0), Point::new(980.0, 320.0)),
            progress_bar: Rect::from_min_max(Point::new(580.0, 190.0), Point::new(960.0, 200.0)),
            cancel_button: Rect::from_min_max(Point::new(876.0, 282.0), Point::new(960.0, 302.0)),
        };
        let layout = compute_progress_overlay_text_layout(sections, style.sizing, true, true);
        assert_inside(sections.dialog, layout.title);
        assert_inside(sections.dialog, layout.detail.expect("detail row"));
        assert_inside(sections.dialog, layout.counter);
        assert_inside(sections.cancel_button, layout.cancel_label);
        assert!(layout.counter.min.y >= sections.progress_bar.max.y);
    }

    #[test]
    fn drag_text_layout_stays_inside_banner() {
        let style = StyleTokens::for_viewport_width(1280.0);
        let banner = Rect::from_min_max(Point::new(420.0, 620.0), Point::new(900.0, 656.0));
        let layout = compute_drag_overlay_text_layout(banner, style.sizing);
        assert_inside(banner, layout.label);
    }
}
