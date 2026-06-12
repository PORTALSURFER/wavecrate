use wavecrate::selection::SelectionRange;

use crate::native_app::app::NativeAppState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct ResolvedPlaybackSpan {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) offset_ratio: f32,
}

pub(super) fn loop_retarget_offset_for_selection(playhead: f32, selection: SelectionRange) -> f32 {
    let start = selection.start();
    let end = selection.end();
    if (start..=end).contains(&playhead) {
        playhead
    } else {
        start
    }
}

pub(super) fn playback_span_matches_selection(
    span: Option<(f32, f32)>,
    selection: SelectionRange,
) -> bool {
    let Some((start, end)) = span else {
        return false;
    };
    (start - selection.start()).abs() <= 0.000_1 && (end - selection.end()).abs() <= 0.000_1
}

impl NativeAppState {
    pub(in crate::native_app) fn resolve_playback_span(
        &self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> ResolvedPlaybackSpan {
        let requested_start = start_ratio.clamp(0.0, 1.0);
        let requested_end = end_ratio.clamp(requested_start, 1.0);
        if !self.audio.loop_playback {
            return ResolvedPlaybackSpan {
                start_ratio: requested_start,
                end_ratio: requested_end,
                offset_ratio: requested_start,
            };
        }

        let (loop_start, loop_end) = self
            .waveform
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        let start_ratio = loop_start.clamp(0.0, 1.0);
        let end_ratio = loop_end.clamp(start_ratio, 1.0);
        let requested_offset = loop_offset_ratio.unwrap_or(requested_start).clamp(0.0, 1.0);
        let offset_ratio = if (start_ratio..=end_ratio).contains(&requested_offset) {
            requested_offset
        } else {
            start_ratio
        };

        ResolvedPlaybackSpan {
            start_ratio,
            end_ratio,
            offset_ratio,
        }
    }
}
