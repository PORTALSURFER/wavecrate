use super::{
    WaveformEditFadeHandle,
    edit_fade_resize::{
        resize_fade_in_end_with_collision, resize_fade_in_outer_start, resize_fade_in_start,
        resize_fade_out_end, resize_fade_out_outer_end, resize_fade_out_start_with_collision,
    },
};

#[derive(Clone, Copy, Debug)]
pub(in crate::gui_app) struct WaveformEditFadeDrag {
    pub(in crate::gui_app) handle: WaveformEditFadeHandle,
    pub(in crate::gui_app) fixed_ratio: f32,
    pub(in crate::gui_app) curve: f32,
    pub(in crate::gui_app) baseline: wavecrate::selection::SelectionRange,
}

impl WaveformEditFadeDrag {
    pub(in crate::gui_app) fn new(
        handle: WaveformEditFadeHandle,
        selection: wavecrate::selection::SelectionRange,
    ) -> Self {
        let curve = match handle {
            WaveformEditFadeHandle::FadeInEnd
            | WaveformEditFadeHandle::FadeInStart
            | WaveformEditFadeHandle::FadeInOuterStart => {
                selection.fade_in().map(|fade| fade.curve).unwrap_or(0.5)
            }
            WaveformEditFadeHandle::FadeOutStart
            | WaveformEditFadeHandle::FadeOutEnd
            | WaveformEditFadeHandle::FadeOutOuterEnd => {
                selection.fade_out().map(|fade| fade.curve).unwrap_or(0.5)
            }
        };
        let fixed_ratio = match handle {
            WaveformEditFadeHandle::FadeInStart => selection
                .fade_in()
                .map(|fade| selection.start() + selection.width() * fade.length)
                .unwrap_or(selection.start()),
            WaveformEditFadeHandle::FadeOutEnd => selection
                .fade_out()
                .map(|fade| selection.end() - selection.width() * fade.length)
                .unwrap_or(selection.end()),
            WaveformEditFadeHandle::FadeInEnd
            | WaveformEditFadeHandle::FadeOutStart
            | WaveformEditFadeHandle::FadeInOuterStart
            | WaveformEditFadeHandle::FadeOutOuterEnd => 0.0,
        };
        Self {
            handle,
            fixed_ratio,
            curve,
            baseline: selection,
        }
    }

    pub(in crate::gui_app) fn apply(
        self,
        selection: wavecrate::selection::SelectionRange,
        ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.handle {
            WaveformEditFadeHandle::FadeInEnd => {
                resize_fade_in_end_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutStart => {
                resize_fade_out_start_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeInStart => {
                resize_fade_in_start(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutEnd => {
                resize_fade_out_end(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeInOuterStart => {
                resize_fade_in_outer_start(selection, ratio)
            }
            WaveformEditFadeHandle::FadeOutOuterEnd => resize_fade_out_outer_end(selection, ratio),
        }
    }
}
