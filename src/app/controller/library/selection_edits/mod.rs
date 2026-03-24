use super::*;
use crate::app::state::DestructiveSelectionEdit;
use std::time::Duration;

mod buffer;
mod controller_actions;
mod controller_apply;
mod ops;
mod prompt;
mod undo_entries;
mod write_service;

mod selection_click;
mod selection_normalize;

pub(crate) use buffer::next_crop_relative_path;
use buffer::{SelectionEditBuffer, SelectionTarget};
pub(crate) use selection_click::repair_clicks_selection as repair_clicks_buffer;
use selection_normalize::normalize_selection;
use write_service::{SelectionEditWriteRequest, apply_selection_edit_write};

use ops::{
    SelectionFadeRequest, apply_directional_fade, apply_edge_fades, apply_selection_fades,
    crop_buffer, reverse_buffer, trim_buffer,
};

#[cfg(test)]
use buffer::selection_frame_bounds;
#[cfg(test)]
use ops::{apply_muted_selection, fade_factor, slice_frames};

use crate::app::controller::undo;

/// Direction of a fade applied over the active selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FadeDirection {
    /// Fade from full level at the left edge to silence at the right edge.
    LeftToRight,
    /// Fade from silence at the left edge to full level at the right edge.
    RightToLeft,
}

/// Result of a destructive edit request.
pub(crate) enum SelectionEditRequest {
    Applied,
    Prompted,
}

fn selection_target_range(
    edit_selection: Option<SelectionRange>,
    play_selection: Option<SelectionRange>,
) -> SelectionRange {
    let edit_selection = edit_selection.filter(|range| range.width() > 0.0);
    let play_selection = play_selection.filter(|range| range.width() > 0.0);
    edit_selection
        .or(play_selection)
        .unwrap_or_else(|| SelectionRange::new(0.0, 1.0))
}

/// Apply short edge fades across an entire clip, returning true when applied.
pub(crate) fn apply_short_edge_fades_to_clip(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    fade_duration: Duration,
) -> bool {
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return false;
    }
    let fade_frames = edge_fade_frame_count(sample_rate.max(1), total_frames, fade_duration);
    if fade_frames == 0 {
        return false;
    }
    apply_edge_fades(samples, channels, 0, total_frames, fade_frames);
    true
}

fn edge_fade_frame_count(sample_rate: u32, selection_frames: usize, duration: Duration) -> usize {
    if selection_frames == 0 {
        return 0;
    }
    let frames = (sample_rate as f32 * duration.as_secs_f32()).round() as usize;
    frames.min(selection_frames / 2)
}

#[cfg(test)]
#[path = "../selection_edits_tests.rs"]
mod selection_edits_tests;
