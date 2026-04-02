//! Request planning and cache-backed metadata lookups for drop-target transfers.

use crate::app::controller::ui::drag_drop_controller::DragDropController;
use crate::app::controller::jobs::{DropTargetTransferMetadata, DropTargetTransferRequest};
use crate::app::state::DragSample;
use crate::sample_sources::SampleSource;
use std::path::Path;

impl DragDropController<'_> {
    /// Resolve drag samples into worker requests and collect preflight validation errors.
    pub(super) fn collect_drop_target_transfer_requests(
        &mut self,
        samples: &[DragSample],
    ) -> (Vec<DropTargetTransferRequest>, Vec<String>) {
        let mut requests = Vec::new();
        let mut errors = Vec::new();
        for sample in samples {
            let Some(source) = self.lookup_drop_target_source(sample) else {
                errors.push(format!(
                    "Source not available for drop: {}",
                    sample.relative_path.display()
                ));
                continue;
            };
            if sample.relative_path.file_name().is_none() {
                errors.push("Sample name unavailable for drop".to_string());
                continue;
            }
            if !source.root.join(&sample.relative_path).exists() {
                errors.push(format!("File missing: {}", sample.relative_path.display()));
                continue;
            }
            requests.push(DropTargetTransferRequest {
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                relative_path: sample.relative_path.clone(),
                metadata: self.cached_drop_target_metadata(&source, &sample.relative_path),
            });
        }
        (requests, errors)
    }

    fn lookup_drop_target_source(&self, sample: &DragSample) -> Option<SampleSource> {
        self.library
            .sources
            .iter()
            .find(|source| source.id == sample.source_id)
            .cloned()
    }

    /// Read cache-backed metadata for a dragged sample without falling back to source DB I/O.
    fn cached_drop_target_metadata(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<DropTargetTransferMetadata> {
        self.cached_source_entry_metadata(source, relative_path)
            .or_else(|| self.cached_selected_source_metadata(source, relative_path))
    }

    fn cached_source_entry_metadata(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<DropTargetTransferMetadata> {
        let cache = self.cache.wav.entries.get(&source.id)?;
        let index = cache.lookup.get(relative_path).copied()?;
        let entry = cache.entry(index)?;
        Some(drop_target_metadata(entry))
    }

    fn cached_selected_source_metadata(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<DropTargetTransferMetadata> {
        if self.selection_state.ctx.selected_source.as_ref() != Some(&source.id) {
            return None;
        }
        let index = self.wav_entries.lookup.get(relative_path).copied()?;
        let entry = self.wav_entries.entry(index)?;
        Some(drop_target_metadata(entry))
    }
}

fn drop_target_metadata(entry: &crate::sample_sources::WavEntry) -> DropTargetTransferMetadata {
    DropTargetTransferMetadata {
        tag: entry.tag,
        looped: entry.looped,
        locked: entry.locked,
        last_played_at: entry.last_played_at,
    }
}
