use super::*;

impl FolderProjectionRuntime {
    /// Begin one pane-scoped projection request and supersede any older request for the same pane.
    pub(crate) fn begin(
        &mut self,
        request_id: u64,
        pane: FolderPaneId,
        source_id: SourceId,
        queued_at: Instant,
    ) {
        self.pending.insert(
            pane,
            PendingFolderProjection {
                request_id,
                pane,
                source_id,
                queued_at,
            },
        );
    }

    /// Return whether a completion still owns the latest request for its pane.
    pub(crate) fn matches(
        &self,
        pane: FolderPaneId,
        source_id: &SourceId,
        request_id: u64,
    ) -> bool {
        self.pending.get(&pane).is_some_and(|pending| {
            pending.request_id == request_id
                && pending.source_id == *source_id
                && pending.pane == pane
        })
    }

    /// Finish a completion only when it still owns the latest request for its pane.
    pub(crate) fn finish_matching(
        &mut self,
        pane: FolderPaneId,
        source_id: &SourceId,
        request_id: u64,
    ) -> bool {
        if !self.matches(pane, source_id, request_id) {
            return false;
        }
        self.pending.remove(&pane);
        true
    }

    /// Cancel any in-flight projection request owned by one pane.
    pub(crate) fn cancel_pane(&mut self, pane: FolderPaneId) {
        self.pending.remove(&pane);
    }

    /// Cancel all in-flight folder projection requests.
    pub(crate) fn cancel_all(&mut self) {
        self.pending.clear();
    }

    /// Return whether one pane is awaiting a folder projection completion.
    pub(crate) fn is_pending(&self, pane: FolderPaneId) -> bool {
        self.pending.contains_key(&pane)
    }

    #[cfg(test)]
    /// Return the pending projection request for one pane.
    pub(crate) fn pending_for_tests(&self, pane: FolderPaneId) -> Option<&PendingFolderProjection> {
        self.pending.get(&pane)
    }
}
