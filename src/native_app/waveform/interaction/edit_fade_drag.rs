use super::{
    WaveformEditFadeHandle,
    edit_fade_resize::{
        resize_fade_in_end_with_collision, resize_fade_in_outer_start, resize_fade_in_start,
        resize_fade_out_end, resize_fade_out_outer_end, resize_fade_out_start_with_collision,
    },
};

#[derive(Clone, Copy, Debug)]
pub(in crate::native_app) struct WaveformEditFadeDrag {
    pub(in crate::native_app) handle: WaveformEditFadeHandle,
    pub(in crate::native_app) fixed_ratio: f32,
    pub(in crate::native_app) curve: f32,
    pub(in crate::native_app) baseline: wavecrate::selection::SelectionRange,
}

impl WaveformEditFadeDrag {
    pub(in crate::native_app) fn new(
        handle: WaveformEditFadeHandle,
        selection: wavecrate::selection::SelectionRange,
    ) -> Self {
        let curve = match handle {
            WaveformEditFadeHandle::InEnd
            | WaveformEditFadeHandle::InStart
            | WaveformEditFadeHandle::InOuterStart => {
                selection.fade_in().map(|fade| fade.curve).unwrap_or(0.5)
            }
            WaveformEditFadeHandle::OutStart
            | WaveformEditFadeHandle::OutEnd
            | WaveformEditFadeHandle::OutOuterEnd => {
                selection.fade_out().map(|fade| fade.curve).unwrap_or(0.5)
            }
        };
        let fixed_ratio = match handle {
            WaveformEditFadeHandle::InStart => selection
                .fade_in()
                .map(|fade| selection.start() + selection.width() * fade.length)
                .unwrap_or(selection.start()),
            WaveformEditFadeHandle::OutEnd => selection
                .fade_out()
                .map(|fade| selection.end() - selection.width() * fade.length)
                .unwrap_or(selection.end()),
            WaveformEditFadeHandle::InEnd
            | WaveformEditFadeHandle::OutStart
            | WaveformEditFadeHandle::InOuterStart
            | WaveformEditFadeHandle::OutOuterEnd => 0.0,
        };
        Self {
            handle,
            fixed_ratio,
            curve,
            baseline: selection,
        }
    }

    pub(in crate::native_app) fn apply(
        self,
        selection: wavecrate::selection::SelectionRange,
        ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.handle {
            WaveformEditFadeHandle::InEnd => {
                resize_fade_in_end_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::OutStart => {
                resize_fade_out_start_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::InStart => {
                resize_fade_in_start(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::OutEnd => {
                resize_fade_out_end(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::InOuterStart => resize_fade_in_outer_start(selection, ratio),
            WaveformEditFadeHandle::OutOuterEnd => resize_fade_out_outer_end(selection, ratio),
        }
    }
}
