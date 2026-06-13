use std::borrow::Cow;

use super::super::fade_preview::{
    apply_fade_to_columns, apply_fade_to_samples, fade_intersects_view,
};
use super::super::plan::PlannedViewport;

pub(super) fn line_render_samples<'a>(
    samples: &'a [f32],
    channels: usize,
    viewport: PlannedViewport,
) -> Cow<'a, [f32]> {
    if viewport.edit_fade.is_some()
        && fade_intersects_view(viewport.view_start, viewport.view_end, viewport.edit_fade)
    {
        return Cow::Owned(apply_fade_to_samples(
            samples,
            channels,
            samples.len() / channels,
            viewport.view_start,
            viewport.view_end,
            viewport.edit_fade,
        ));
    }
    Cow::Borrowed(samples)
}

pub(super) fn apply_fade_to_column_model(
    columns: &mut [(f32, f32)],
    view_start: f32,
    view_end: f32,
    edit_fade: Option<crate::selection::SelectionRange>,
) {
    apply_fade_to_columns(columns, view_start, view_end, edit_fade);
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::line_render_samples;
    use crate::selection::SelectionRange;
    use crate::waveform::render::plan::PlannedViewport;

    #[test]
    fn line_render_samples_borrows_when_fade_is_absent() {
        let samples = [0.0, 0.25, -0.25, 0.5];
        let viewport = PlannedViewport {
            width: 4,
            height: 5,
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: None,
        };

        let line_samples = line_render_samples(&samples, 1, viewport);

        assert!(matches!(line_samples, Cow::Borrowed(_)));
        assert!(std::ptr::eq(
            line_samples.as_ref().as_ptr(),
            samples.as_ptr()
        ));
    }

    #[test]
    fn line_render_samples_owns_when_fade_intersects_view() {
        let samples = [1.0, 1.0, 1.0, 1.0];
        let viewport = PlannedViewport {
            width: 4,
            height: 5,
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: Some(SelectionRange::new(0.0, 1.0).with_fade_out(1.0, 0.0)),
        };

        let line_samples = line_render_samples(&samples, 1, viewport);

        assert!(matches!(line_samples, Cow::Owned(_)));
    }
}
