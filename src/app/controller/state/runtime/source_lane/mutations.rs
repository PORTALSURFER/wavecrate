use super::*;

impl SourceMutationRuntime {
    /// Track one optimistic metadata request and its affected relative paths.
    pub(crate) fn insert_metadata_mutation(&mut self, pending: PendingMetadataMutation) {
        let source_id = pending.source_id.clone();
        for path in &pending.paths {
            self.pending_metadata_paths
                .insert((source_id.clone(), path.clone()));
            if pending.blocks_file_mutation {
                self.pending_file_blocking_metadata_paths
                    .insert((source_id.clone(), path.clone()));
            }
        }
        self.pending_metadata_mutations
            .insert(pending.request_id, pending);
    }

    #[cfg(test)]
    /// Return the number of metadata mutation requests currently awaiting completion.
    pub(crate) fn pending_metadata_count(&self) -> usize {
        self.pending_metadata_mutations.len()
    }

    /// Remove one metadata request and clear its tracked pending paths.
    pub(crate) fn finish_metadata_mutation(
        &mut self,
        request_id: u64,
    ) -> Option<PendingMetadataMutation> {
        let pending = self.pending_metadata_mutations.remove(&request_id)?;
        for path in &pending.paths {
            self.pending_metadata_paths
                .remove(&(pending.source_id.clone(), path.clone()));
            if pending.blocks_file_mutation {
                self.pending_file_blocking_metadata_paths
                    .remove(&(pending.source_id.clone(), path.clone()));
            }
        }
        Some(pending)
    }

    /// Return whether one sample path still has an optimistic metadata write in flight.
    pub(crate) fn metadata_path_pending(
        &self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
    ) -> bool {
        self.pending_file_blocking_metadata_paths
            .contains(&(source_id.clone(), relative_path.to_path_buf()))
    }

    /// Return whether any optimistic metadata write is pending for `source_id`.
    pub(crate) fn source_has_pending_metadata(&self, source_id: &SourceId) -> bool {
        self.pending_metadata_paths
            .iter()
            .any(|(pending_source_id, _)| pending_source_id == source_id)
    }

    /// Extend the source-scoped claim-pause grace window through `until`.
    pub(crate) fn extend_claim_pause_grace(&mut self, source_id: &SourceId, until: Instant) {
        let entry = self
            .claim_pause_grace_until
            .entry(source_id.clone())
            .or_insert(until);
        if *entry < until {
            *entry = until;
        }
    }

    /// Return whether the source-scoped claim-pause grace window is still active.
    pub(crate) fn claim_pause_grace_active(&mut self, source_id: &SourceId, now: Instant) -> bool {
        match self.claim_pause_grace_until.get(source_id).copied() {
            Some(until) if until > now => true,
            Some(_) => {
                self.claim_pause_grace_until.remove(source_id);
                false
            }
            None => false,
        }
    }

    /// Extend the source-scoped auto-sync suppression window through `until`.
    pub(crate) fn extend_auto_sync_grace(&mut self, source_id: &SourceId, until: Instant) {
        let entry = self
            .auto_sync_grace_until
            .entry(source_id.clone())
            .or_insert(until);
        if *entry < until {
            *entry = until;
        }
    }

    /// Return whether the source-scoped auto-sync suppression window is active.
    pub(crate) fn auto_sync_grace_active(&mut self, source_id: &SourceId, now: Instant) -> bool {
        match self.auto_sync_grace_until.get(source_id).copied() {
            Some(until) if until > now => true,
            Some(_) => {
                self.auto_sync_grace_until.remove(source_id);
                false
            }
            None => false,
        }
    }

    /// Return whether one source currently owns a background file mutation.
    pub(crate) fn source_has_pending_file_mutations(&self, source_id: &SourceId) -> bool {
        self.pending_file_mutation_sources.contains(source_id)
    }

    /// Mark one source/path batch as owned by a background file mutation.
    pub(crate) fn begin_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> bool {
        let source_was_inactive = self.pending_file_mutation_sources.insert(source_id.clone());
        for path in paths {
            self.pending_file_mutation_paths
                .insert((source_id.clone(), path));
        }
        source_was_inactive
    }

    /// Clear one source/path batch from background file-mutation tracking.
    pub(crate) fn finish_file_mutation(
        &mut self,
        source_id: &SourceId,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> bool {
        for path in paths {
            self.pending_file_mutation_paths
                .remove(&(source_id.clone(), path));
        }
        let still_has_paths = self
            .pending_file_mutation_paths
            .iter()
            .any(|(pending_source_id, _)| pending_source_id == source_id);
        if !still_has_paths {
            return self.pending_file_mutation_sources.remove(source_id);
        }
        false
    }

    /// Mark one browser rename or auto-rename intent as the file-op lane owner.
    pub(crate) fn begin_browser_rename_intent(&mut self, key: BrowserRenameIntentKey) {
        self.active_browser_rename_intent = Some(key);
    }

    /// Drop the active browser rename owner and return any queued follow-up.
    pub(crate) fn finish_browser_rename_intent(
        &mut self,
    ) -> Option<PendingBrowserAutoRenameIntent> {
        self.active_browser_rename_intent = None;
        self.active_auto_rename_batch = None;
        self.queued_browser_auto_rename_intent.take()
    }

    /// Forget active browser rename ownership when dispatch fails before work starts.
    pub(crate) fn clear_browser_rename_intent(&mut self) {
        self.active_browser_rename_intent = None;
        self.active_auto_rename_batch = None;
    }

    /// Apply the OPT-135 policy for repeat auto-rename input while a file op is active.
    pub(crate) fn handle_busy_browser_auto_rename_intent(
        &mut self,
        key: BrowserRenameIntentKey,
        pending: PendingBrowserAutoRenameIntent,
    ) -> BrowserRenameBusyDecision {
        let Some(active) = self.active_browser_rename_intent.as_ref() else {
            return BrowserRenameBusyDecision::UnrelatedFileOp;
        };
        if active == &key
            || self
                .queued_browser_auto_rename_intent
                .as_ref()
                .is_some_and(|queued| queued.key == key)
        {
            return BrowserRenameBusyDecision::Collapsed;
        }
        self.queued_browser_auto_rename_intent = Some(pending);
        BrowserRenameBusyDecision::Queued
    }

    /// Return whether the same browser rename request is already active.
    pub(crate) fn browser_rename_intent_is_active(&self, key: &BrowserRenameIntentKey) -> bool {
        self.active_browser_rename_intent.as_ref() == Some(key)
    }

    /// Record the latest optimistic Loop/One-shot edit for one source/path.
    pub(crate) fn begin_looped_metadata_intent(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
    ) -> u64 {
        self.next_looped_metadata_intent_id =
            self.next_looped_metadata_intent_id.saturating_add(1).max(1);
        let intent_id = self.next_looped_metadata_intent_id;
        self.looped_metadata_intents
            .insert((source_id.clone(), relative_path.to_path_buf()), intent_id);
        intent_id
    }

    /// Return whether a Loop/One-shot completion still owns the current optimistic edit.
    pub(crate) fn looped_metadata_intent_matches(
        &self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
    ) -> bool {
        self.looped_metadata_intents
            .get(&(source_id.clone(), relative_path.to_path_buf()))
            .copied()
            == Some(intent_id)
    }

    /// Clear a completed Loop/One-shot intent only if it is still the current owner.
    pub(crate) fn finish_looped_metadata_intent(
        &mut self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
    ) {
        let key = (source_id.clone(), relative_path.to_path_buf());
        if self.looped_metadata_intents.get(&key).copied() == Some(intent_id) {
            self.looped_metadata_intents.remove(&key);
        }
    }

    /// Follow a successful rename so stale metadata completions check the live path.
    pub(crate) fn remap_looped_metadata_intent(
        &mut self,
        source_id: &SourceId,
        old_relative: &std::path::Path,
        new_relative: &std::path::Path,
    ) {
        if old_relative == new_relative {
            return;
        }
        let old_key = (source_id.clone(), old_relative.to_path_buf());
        if let Some(intent_id) = self.looped_metadata_intents.remove(&old_key) {
            self.looped_metadata_intents
                .insert((source_id.clone(), new_relative.to_path_buf()), intent_id);
        }
    }
}
