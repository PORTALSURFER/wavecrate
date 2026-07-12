use super::*;
use std::path::{Path, PathBuf};
use std::time::Instant;

impl AppController {
    pub(crate) fn cancel_pending_source_remap_for_mutation(&mut self, source_id: &SourceId) {
        let Some(pending) = self.runtime.source_lane.pending_remap.as_mut() else {
            return;
        };
        if pending.source.id != *source_id || pending.canceled {
            return;
        }
        pending.canceled = true;
        pending.write_fence.cancel();
        tracing::info!(
            source_id = %source_id,
            request_id = pending.request_id,
            "Canceling pending source remap before source mutation"
        );
    }

    pub(crate) fn selected_source_claim_pause_grace_active(&mut self, now: Instant) -> bool {
        let Some(source_id) = self.selected_source_id() else {
            return false;
        };
        self.runtime
            .source_lane
            .mutations
            .claim_pause_grace_active(&source_id, now)
    }

    pub(crate) fn extend_selected_source_mutation_claim_grace(&mut self, source_id: &SourceId) {
        if self.selected_source_id().as_ref() != Some(source_id) {
            return;
        }
        self.runtime.source_lane.mutations.extend_claim_pause_grace(
            source_id,
            Instant::now() + SELECTED_SOURCE_MUTATION_CLAIM_GRACE,
        );
    }

    pub(crate) fn selected_source_auto_sync_grace_active(&mut self, now: Instant) -> bool {
        let Some(source_id) = self.selected_source_id() else {
            return false;
        };
        self.runtime
            .source_lane
            .mutations
            .auto_sync_grace_active(&source_id, now)
    }

    pub(crate) fn source_auto_sync_grace_active(
        &mut self,
        source_id: &SourceId,
        now: Instant,
    ) -> bool {
        self.runtime
            .source_lane
            .mutations
            .auto_sync_grace_active(source_id, now)
    }

    pub(crate) fn extend_selected_source_mutation_auto_sync_grace(&mut self, source_id: &SourceId) {
        if self.selected_source_id().as_ref() != Some(source_id) {
            return;
        }
        self.runtime.source_lane.mutations.extend_auto_sync_grace(
            source_id,
            Instant::now() + SELECTED_SOURCE_MUTATION_AUTO_SYNC_GRACE,
        );
    }

    /// Return whether one sample path already has an optimistic metadata write in flight.
    pub(crate) fn metadata_mutation_pending_for(
        &self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> bool {
        self.runtime
            .source_lane
            .mutations
            .metadata_path_pending(source_id, relative_path)
    }

    /// Return whether the currently selected source still has optimistic metadata writes pending.
    pub(crate) fn selected_source_has_pending_metadata_mutations(&self) -> bool {
        self.selected_source_id().is_some_and(|source_id| {
            self.runtime
                .source_lane
                .mutations
                .source_has_pending_metadata(&source_id)
        })
    }

    /// Return whether the currently selected source still owns a background file mutation.
    pub(crate) fn selected_source_has_pending_file_mutations(&self) -> bool {
        self.selected_source_id().is_some_and(|source_id| {
            self.runtime
                .source_lane
                .mutations
                .source_has_pending_file_mutations(&source_id)
        })
    }

    /// Return whether one source currently owns a background file mutation.
    pub(crate) fn source_has_pending_file_mutations(&self, source_id: &SourceId) -> bool {
        self.runtime
            .source_lane
            .mutations
            .source_has_pending_file_mutations(source_id)
    }

    /// Mark one source/path batch as owned by a background file mutation.
    pub(crate) fn begin_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        self.cancel_pending_source_remap_for_mutation(source_id);
        let paths = paths.into_iter().collect::<Vec<_>>();
        let source_became_active = self
            .runtime
            .source_lane
            .mutations
            .begin_file_mutation(source_id, paths.clone());
        if source_became_active {
            crate::app::controller::library::source_write_priority::begin_file_op_write_priority(
                source_id,
            );
        }
        self.runtime
            .jobs
            .begin_source_watch_file_op(source_id.clone(), paths);
        self.extend_selected_source_mutation_claim_grace(source_id);
        self.extend_selected_source_mutation_auto_sync_grace(source_id);
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_pending_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) {
        let paths = paths.into_iter().collect::<Vec<_>>();
        let source_became_inactive = self
            .runtime
            .source_lane
            .mutations
            .finish_file_mutation(source_id, paths.clone());
        if source_became_inactive {
            crate::app::controller::library::source_write_priority::finish_file_op_write_priority(
                source_id,
            );
        }
        self.runtime
            .jobs
            .finish_source_watch_file_op(source_id.clone(), paths);
        self.extend_selected_source_mutation_claim_grace(source_id);
        self.extend_selected_source_mutation_auto_sync_grace(source_id);
    }
}
