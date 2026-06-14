use super::super::{UiPoint, controls::DestructiveEditPrompt};
use crate::selection::SelectionRange;
use crate::waveform::WaveformChannelView;
use std::time::Instant;

pub(super) struct WaveformSelectionState {
    pub(super) cursor: Option<f32>,
    pub(super) selection: Option<SelectionRange>,
    pub(super) last_bpm_grid_origin: f32,
    pub(super) selection_duration: Option<String>,
    pub(super) edit_selection: Option<SelectionRange>,
    pub(super) hover_time_label: Option<String>,
    pub(super) channel_view: WaveformChannelView,
    pub(super) view: WaveformView,
    pub(super) pending_destructive: Option<DestructiveEditPrompt>,
    pub(super) cursor_last_hover_at: Option<Instant>,
    pub(super) cursor_last_navigation_at: Option<Instant>,
    pub(super) hover_pointer_pos: Option<UiPoint>,
    pub(super) hover_pointer_last_moved_at: Option<Instant>,
    pub(super) suppress_hover_cursor: bool,
    pub(super) pan_drag_pos: Option<UiPoint>,
}

impl Default for WaveformSelectionState {
    fn default() -> Self {
        Self {
            cursor: None,
            selection: None,
            last_bpm_grid_origin: 0.0,
            selection_duration: None,
            edit_selection: None,
            hover_time_label: None,
            channel_view: WaveformChannelView::Mono,
            view: WaveformView::default(),
            pending_destructive: None,
            cursor_last_hover_at: None,
            cursor_last_navigation_at: None,
            hover_pointer_pos: None,
            hover_pointer_last_moved_at: None,
            suppress_hover_cursor: false,
            pan_drag_pos: None,
        }
    }
}

/// Normalized bounds describing the visible region of the waveform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformView {
    /// Normalized view start (0.0-1.0).
    pub start: f64,
    /// Normalized view end (0.0-1.0).
    pub end: f64,
}

impl WaveformView {
    /// Clamp the view to a valid range while keeping the width positive.
    pub fn clamp(mut self) -> Self {
        let width = (self.end - self.start).clamp(1e-9, 1.0);
        let start = self.start.clamp(0.0, 1.0 - width);
        self.start = start;
        self.end = (start + width).min(1.0);
        self
    }

    /// Width of the viewport.
    pub fn width(&self) -> f64 {
        (self.end - self.start).max(1e-9)
    }
}

impl Default for WaveformView {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
        }
    }
}
