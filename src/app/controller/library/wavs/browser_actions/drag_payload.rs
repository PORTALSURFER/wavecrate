//! Browser-row drag payload helpers shared by pointer drag/drop and folder hotkeys.
//!
//! The browser selection contract is path-authoritative, but move operations need
//! stable row ordering and drag payload labels. These helpers keep the
//! selected-or-row behavior in one place so browser-origin drag/drop and folder
//! hotkeys resolve the same sample set.

use super::*;
#[cfg(test)]
use crate::app::state::DragPayload;
use crate::app::state::{DragSample, UiPoint};
use crate::app::view_model;

/// Browser-origin sample payload resolved from one primary row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowserSampleDragPayload {
    /// Sample rows included in the drag payload.
    pub samples: Vec<DragSample>,
    /// Display label shown in the drag overlay.
    pub label: String,
}

impl AppController {
    /// Return the visible browser rows covered by the current selection or focus.
    pub(crate) fn browser_selection_rows_for_drag_samples(&mut self) -> Vec<usize> {
        let selected_paths = self.ui.browser.selection.selected_paths.clone();
        let mut rows: Vec<usize> = selected_paths
            .iter()
            .filter_map(|path| self.visible_row_for_path(path))
            .collect();
        if rows.is_empty()
            && let Some(row) = self.focused_browser_row()
        {
            rows.push(row);
        }
        rows.sort_unstable();
        rows.dedup();
        rows
    }

    /// Resolve drag-sample references for a set of visible browser rows.
    pub(crate) fn drag_samples_for_browser_rows(
        &mut self,
        source: &SampleSource,
        rows: &[usize],
    ) -> Vec<DragSample> {
        rows.iter()
            .filter_map(|row| {
                let entry_index = self.visible_browser_index(*row)?;
                let entry = self.wav_entry(entry_index)?;
                Some(DragSample {
                    source_id: source.id.clone(),
                    relative_path: entry.relative_path.clone(),
                })
            })
            .collect()
    }

    /// Resolve the browser drag payload for one pressed visible row.
    ///
    /// If the pressed row is already part of the active multi-selection, the
    /// payload includes the full selected set. Otherwise it includes only the
    /// pressed row.
    pub(crate) fn browser_sample_drag_payload_for_row(
        &mut self,
        visible_row: usize,
    ) -> Option<BrowserSampleDragPayload> {
        let source = self.current_source()?;
        let entry_index = self.visible_browser_index(visible_row)?;
        let primary_path = self
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())?;
        let rows = if self
            .ui
            .browser
            .selection
            .selected_paths
            .iter()
            .any(|path| path == &primary_path)
        {
            self.browser_selection_rows_for_drag_samples()
        } else {
            vec![visible_row]
        };
        let samples = self.drag_samples_for_browser_rows(&source, &rows);
        let label = browser_sample_drag_label(&samples)?;
        Some(BrowserSampleDragPayload { samples, label })
    }

    /// Start a browser-origin sample drag from one visible row.
    pub(crate) fn start_browser_sample_drag_action(&mut self, visible_row: usize, pos: UiPoint) {
        let Some(payload) = self.browser_sample_drag_payload_for_row(visible_row) else {
            return;
        };
        match payload.samples.as_slice() {
            [sample] => self.start_sample_drag(
                sample.source_id.clone(),
                sample.relative_path.clone(),
                payload.label,
                pos,
            ),
            _ => self.start_samples_drag(payload.samples, payload.label, pos),
        }
    }

    /// Convert one resolved browser drag payload into the drag-drop payload enum.
    #[cfg(test)]
    pub(crate) fn browser_drag_payload_for_tests(
        &mut self,
        visible_row: usize,
    ) -> Option<DragPayload> {
        let payload = self.browser_sample_drag_payload_for_row(visible_row)?;
        Some(match payload.samples.as_slice() {
            [sample] => DragPayload::Sample {
                source_id: sample.source_id.clone(),
                relative_path: sample.relative_path.clone(),
            },
            _ => DragPayload::Samples {
                samples: payload.samples,
            },
        })
    }
}

fn browser_sample_drag_label(samples: &[DragSample]) -> Option<String> {
    match samples {
        [] => None,
        [sample] => Some(view_model::sample_display_label(&sample.relative_path)),
        many => Some(format!("{} samples", many.len())),
    }
}
