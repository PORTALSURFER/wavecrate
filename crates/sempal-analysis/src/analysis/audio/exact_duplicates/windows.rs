/// One candidate duplicate-detection window in decoded sample space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Window {
    pub(super) start_frame: usize,
    pub(super) end_frame: usize,
}

pub(super) fn selection_window(
    total_frames: usize,
    window_frames: usize,
    anchor_start_frame: usize,
) -> Option<Window> {
    if window_frames == 0 || window_frames > total_frames {
        return None;
    }
    let start_frame = anchor_start_frame.min(total_frames.saturating_sub(window_frames));
    Some(Window {
        start_frame,
        end_frame: start_frame + window_frames,
    })
}

pub(super) fn selection_overlaps_audible_audio(
    selection_window: Window,
    audible_ranges: &[(usize, usize)],
) -> bool {
    audible_ranges
        .iter()
        .copied()
        .any(|range| range.0 < selection_window.end_frame && range.1 > selection_window.start_frame)
}

pub(super) fn selection_event_frame(
    selection_window: Window,
    audible_ranges: &[(usize, usize)],
    transient_event_frames: &[usize],
) -> usize {
    transient_event_frames
        .iter()
        .copied()
        .filter(|frame| {
            *frame >= selection_window.start_frame && *frame < selection_window.end_frame
        })
        .min_by_key(|frame| frame.saturating_sub(selection_window.start_frame))
        .or_else(|| {
            audible_ranges
                .iter()
                .copied()
                .filter(|range| {
                    range.0 < selection_window.end_frame && range.1 > selection_window.start_frame
                })
                .map(|range| range.0)
                .min_by_key(|frame| frame.abs_diff(selection_window.start_frame))
        })
        .unwrap_or(selection_window.start_frame)
}

pub(super) fn collect_candidate_windows(
    total_frames: usize,
    window_frames: usize,
    selection_window: Window,
    selection_event_offset: isize,
    audible_ranges: &[(usize, usize)],
    transient_event_frames: &[usize],
) -> Vec<Window> {
    let mut event_frames = audible_ranges
        .iter()
        .map(|range| range.0)
        .collect::<Vec<_>>();
    event_frames.extend(
        transient_event_frames
            .iter()
            .copied()
            .filter(|frame| *frame < total_frames),
    );
    event_frames.push(selection_event_frame(
        selection_window,
        audible_ranges,
        transient_event_frames,
    ));
    let mut windows = event_frames
        .into_iter()
        .filter_map(|event_start| {
            candidate_window(
                total_frames,
                window_frames,
                selection_event_offset,
                event_start,
            )
        })
        .filter(|window| window_contains_audible_audio(*window, audible_ranges))
        .collect::<Vec<_>>();
    windows.push(selection_window);
    windows.sort_by_key(|window| window.start_frame);
    windows.dedup_by_key(|window| window.start_frame);
    windows
}

fn candidate_window(
    total_frames: usize,
    window_frames: usize,
    selection_event_offset: isize,
    event_start_frame: usize,
) -> Option<Window> {
    let start_frame = event_start_frame as isize + selection_event_offset;
    if start_frame < 0 {
        return None;
    }
    let start_frame = start_frame as usize;
    let end_frame = start_frame.checked_add(window_frames)?;
    (end_frame <= total_frames).then_some(Window {
        start_frame,
        end_frame,
    })
}

fn window_contains_audible_audio(window: Window, audible_ranges: &[(usize, usize)]) -> bool {
    audible_ranges
        .iter()
        .copied()
        .any(|range| range.0 < window.end_frame && range.1 > window.start_frame)
}
