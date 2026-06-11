use wavecrate::selection::SelectionRange;

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
