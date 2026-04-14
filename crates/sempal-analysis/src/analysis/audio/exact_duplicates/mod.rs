use super::detect_non_silent_ranges_for_slices;

mod matching;
mod windows;

#[cfg(test)]
mod tests;

use matching::collect_duplicate_windows;
use windows::{
    collect_candidate_windows, selection_event_frame, selection_overlaps_audible_audio,
    selection_window,
};

const PEAK_RATIO_TOLERANCE: f32 = 1.08;
const MEAN_ABS_DIFF_TOLERANCE: f32 = 0.004;
const RMS_DIFF_TOLERANCE: f32 = 0.006;
const MAX_DIFF_TOLERANCE: f32 = 0.035;
const MIN_AUDIBLE_PEAK: f32 = 1.0e-4;

/// One duplicate cleanup preview derived from the loaded waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectedDuplicateWindow {
    /// Window start frame in decoded sample space.
    pub start_frame: usize,
    /// Window end frame in decoded sample space.
    pub end_frame: usize,
    /// Stable duplicate-group id for the matched exemplar family.
    pub group_id: usize,
}

/// Near-duplicate cleanup candidates detected from one loaded waveform.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExactDuplicateWindowDetection {
    /// Number of duplicate groups that have at least one removable clone.
    pub duplicate_group_count: usize,
    /// Duplicate windows kept as one preview per removable clone.
    pub duplicate_windows: Vec<DetectedDuplicateWindow>,
    /// Number of duplicate windows represented by `duplicate_windows`.
    pub duplicate_window_count: usize,
}

/// Detect selection-sized near-duplicate windows across the loaded waveform.
///
/// The caller provides one playback-selection-sized scan window plus optional
/// transient anchors for the current waveform. The detector uses the selection
/// to derive the event offset inside each window, scans candidate hit starts
/// across the whole file, keeps the earliest window for each duplicate group,
/// and returns one removable preview per later near-identical clone.
pub fn detect_exact_duplicate_window_ranges(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    window_frames: usize,
    anchor_start_frame: usize,
    transient_event_frames: &[usize],
) -> Result<ExactDuplicateWindowDetection, String> {
    let channels = channels.max(1) as usize;
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return Ok(ExactDuplicateWindowDetection::default());
    }
    if sample_rate == 0 {
        return Err("Sample rate is unavailable for duplicate cleanup".to_string());
    }
    if window_frames == 0 {
        return Err("Create a playback selection to define the duplicate window size".to_string());
    }
    let Some(selection_window) = selection_window(total_frames, window_frames, anchor_start_frame)
    else {
        return Ok(ExactDuplicateWindowDetection::default());
    };

    let audible_ranges = detect_non_silent_ranges_for_slices(samples, channels as u16, sample_rate);
    if !selection_overlaps_audible_audio(selection_window, &audible_ranges) {
        return Err("The duplicate scan selection must include audible audio".to_string());
    }

    let selection_event_frame =
        selection_event_frame(selection_window, &audible_ranges, transient_event_frames);
    let selection_event_offset =
        selection_window.start_frame as isize - selection_event_frame as isize;
    let windows = collect_candidate_windows(
        total_frames,
        window_frames,
        selection_window,
        selection_event_offset,
        &audible_ranges,
        transient_event_frames,
    );
    if windows.len() < 2 {
        return Ok(ExactDuplicateWindowDetection::default());
    }

    Ok(collect_duplicate_windows(samples, channels, &windows))
}
