//! Selection drag state machine and edge update helpers.

use super::range::SelectionRange;

/// The selection edge being dragged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionEdge {
    /// Adjust the starting edge.
    Start,
    /// Adjust the ending edge.
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DragKind {
    Create { anchor: f32 },
    StartEdge,
    EndEdge,
}

impl From<SelectionEdge> for DragKind {
    fn from(edge: SelectionEdge) -> Self {
        match edge {
            SelectionEdge::Start => DragKind::StartEdge,
            SelectionEdge::End => DragKind::EndEdge,
        }
    }
}

/// Tracks active selection and drag gestures.
#[derive(Default, Debug)]
pub struct SelectionState {
    range: Option<SelectionRange>,
    drag: Option<DragKind>,
}

impl SelectionState {
    /// Create an empty selection state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current selection range, if one exists.
    pub fn range(&self) -> Option<SelectionRange> {
        self.range
    }

    /// True while a drag gesture is active.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Begin creating a new selection from the given anchor point.
    pub fn begin_new(&mut self, position: f32) -> SelectionRange {
        let range = SelectionRange::new(position, position);
        self.range = Some(range);
        self.drag = Some(DragKind::Create { anchor: position });
        range
    }

    /// Begin dragging an existing edge; returns false if no selection is present.
    pub fn begin_edge_drag(&mut self, edge: SelectionEdge) -> bool {
        if self.range.is_none() {
            return false;
        }
        self.drag = Some(edge.into());
        true
    }

    /// Update the active drag with a new cursor position.
    pub fn update_drag(&mut self, position: f32) -> Option<SelectionRange> {
        let drag = self.drag?;
        let next_range = match drag {
            DragKind::Create { anchor } => SelectionRange::new(anchor, position),
            DragKind::StartEdge => {
                let range = self.range?;
                SelectionRange::new(position, range.end())
            }
            DragKind::EndEdge => {
                let range = self.range?;
                SelectionRange::new(range.start(), position)
            }
        };
        self.range = Some(next_range);
        Some(next_range)
    }

    /// Update the active drag, snapping the selection length to a beat-sized step.
    pub fn update_drag_snapped(&mut self, position: f32, beat_step: f32) -> Option<SelectionRange> {
        if !beat_step.is_finite() || beat_step <= 0.0 {
            return self.update_drag(position);
        }
        let drag = self.drag?;
        let step = beat_step.clamp(1.0e-6, 1.0);
        let next_range = match drag {
            DragKind::Create { anchor } => {
                let delta = position - anchor;
                let snapped = anchor + snap_delta(delta, step);
                let clamped = clamp01(snapped);
                let mut range = SelectionRange::new(anchor, clamped);
                if range.width() < step && !(0.0..=1.0).contains(&snapped) {
                    if snapped >= anchor {
                        let end = (anchor + step).min(1.0);
                        let start = (end - step).max(0.0);
                        range = SelectionRange::new(start, end);
                    } else {
                        let start = (anchor - step).max(0.0);
                        let end = (start + step).min(1.0);
                        range = SelectionRange::new(start, end);
                    }
                }
                range
            }
            DragKind::StartEdge => {
                let range = self.range?;
                let delta = range.end() - position;
                let snapped = range.end() - snap_delta(delta, step);
                SelectionRange::new(snapped, range.end())
            }
            DragKind::EndEdge => {
                let range = self.range?;
                let delta = position - range.start();
                let snapped = range.start() + snap_delta(delta, step);
                SelectionRange::new(range.start(), snapped)
            }
        };
        let next_range = match drag {
            DragKind::Create { .. } => {
                if next_range.width() < step {
                    self.range = None;
                    return None;
                }
                next_range
            }
            DragKind::StartEdge => enforce_min_width(next_range, step, SelectionEdge::Start),
            DragKind::EndEdge => enforce_min_width(next_range, step, SelectionEdge::End),
        };
        self.range = Some(next_range);
        Some(next_range)
    }

    /// Clear the active drag, keeping the current range intact.
    pub fn finish_drag(&mut self) {
        self.drag = None;
    }

    /// Remove any active selection; returns true if something changed.
    pub fn clear(&mut self) -> bool {
        let changed = self.range.is_some();
        self.range = None;
        self.drag = None;
        changed
    }

    /// Replace the current selection without marking a drag active.
    pub fn set_range(&mut self, range: Option<SelectionRange>) {
        self.range = range;
        self.drag = None;
    }
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn snap_delta(delta: f32, step: f32) -> f32 {
    if !delta.is_finite() || !step.is_finite() || step <= 0.0 {
        return delta;
    }
    (delta / step).round() * step
}

fn enforce_min_width(
    range: SelectionRange,
    min_width: f32,
    anchor: SelectionEdge,
) -> SelectionRange {
    if range.width() >= min_width {
        return range;
    }
    let step = min_width.clamp(0.0, 1.0);
    match anchor {
        SelectionEdge::Start => {
            let mut end = range.end();
            let mut start = (end - step).max(0.0);
            if (end - start) < step {
                end = (start + step).min(1.0);
                start = (end - step).max(0.0);
            }
            SelectionRange::new(start, end)
        }
        SelectionEdge::End => {
            let mut start = range.start();
            let mut end = (start + step).min(1.0);
            if (end - start) < step {
                start = (end - step).max(0.0);
                end = (start + step).min(1.0);
            }
            SelectionRange::new(start, end)
        }
    }
}
