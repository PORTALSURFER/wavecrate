use super::*;
use crate::app::state::DestructiveSelectionEdit;

mod background;
mod buffer;
mod controller_actions;
mod controller_apply;
mod duplicate_cleanup;
mod ops;
mod prompt;
mod undo_entries;
mod write_service;

mod selection_click;
mod selection_normalize;

use background::SelectionEditWorkerOp;
use buffer::{SelectionEditBuffer, selection_frame_bounds};
pub(crate) use buffer::{SelectionTarget, next_crop_relative_path};
pub(crate) use controller_apply::{PlaybackResumeState, SelectionEditVisualState};
pub(crate) use selection_click::repair_clicks_selection as repair_clicks_buffer;
use selection_normalize::normalize_selection;
use write_service::{SelectionEditWriteRequest, apply_selection_edit_write};

use ops::{
    SelectionFadeRequest, apply_directional_fade, apply_edge_fades, apply_selection_fades,
    crop_buffer, reverse_buffer, trim_buffer,
};

#[cfg(test)]
use ops::{apply_muted_selection, fade_factor, slice_frames};

use crate::app::controller::undo;
pub(crate) use crate::audio::apply_short_edge_fades_to_clip;

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

#[cfg(test)]
#[path = "../selection_edits_tests.rs"]
mod selection_edits_tests;
