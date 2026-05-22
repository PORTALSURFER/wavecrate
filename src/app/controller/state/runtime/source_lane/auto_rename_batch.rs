use super::*;

impl SourceMutationRuntime {
    /// Start tracking one active browser auto-rename batch in requested path order.
    pub(crate) fn begin_auto_rename_batch(&mut self, source_id: SourceId, paths: Vec<PathBuf>) {
        self.active_auto_rename_batch = Some(ActiveAutoRenameBatchState::new(source_id, paths));
    }

    /// Apply one structured auto-rename progress message to the active batch state.
    pub(crate) fn apply_auto_rename_progress(&mut self, progress: SampleAutoRenameProgress) {
        if let Some(batch) = self.active_auto_rename_batch.as_mut() {
            batch.apply_progress(progress);
        }
    }

    /// Clear active auto-rename state when the selected source changes.
    pub(crate) fn clear_auto_rename_batch_for_source_change(
        &mut self,
        selected: Option<&SourceId>,
    ) {
        if self
            .active_auto_rename_batch
            .as_ref()
            .is_some_and(|batch| Some(&batch.source_id) != selected)
        {
            self.active_auto_rename_batch = None;
        }
    }

    /// Return a source-scoped immutable snapshot of the active auto-rename batch.
    pub(crate) fn active_auto_rename_batch_snapshot(
        &self,
    ) -> Option<ActiveAutoRenameBatchSnapshot> {
        self.active_auto_rename_batch
            .as_ref()
            .map(ActiveAutoRenameBatchState::snapshot)
    }
}

impl ActiveAutoRenameBatchState {
    fn new(source_id: SourceId, paths: Vec<PathBuf>) -> Self {
        let states = paths
            .iter()
            .cloned()
            .map(|path| (path, AutoRenameBatchRowState::Queued))
            .collect();
        Self {
            source_id,
            requested_paths: paths,
            states,
            remaps: HashMap::new(),
            current_requested_path: None,
        }
    }

    fn apply_progress(&mut self, progress: SampleAutoRenameProgress) {
        match progress {
            SampleAutoRenameProgress::Active { old_relative } => {
                self.current_requested_path = Some(old_relative.clone());
                self.states
                    .insert(old_relative, AutoRenameBatchRowState::Active);
            }
            SampleAutoRenameProgress::Completed {
                old_relative,
                new_relative,
            } => {
                self.current_requested_path = None;
                if old_relative != new_relative {
                    self.remaps
                        .insert(old_relative.clone(), new_relative.clone());
                }
                self.states
                    .insert(old_relative, AutoRenameBatchRowState::Completed);
            }
            SampleAutoRenameProgress::Skipped { old_relative, .. } => {
                self.current_requested_path = None;
                self.states
                    .insert(old_relative, AutoRenameBatchRowState::Skipped);
            }
            SampleAutoRenameProgress::Failed { old_relative, .. } => {
                self.current_requested_path = None;
                self.states
                    .insert(old_relative, AutoRenameBatchRowState::Failed);
            }
        }
    }

    fn snapshot(&self) -> ActiveAutoRenameBatchSnapshot {
        ActiveAutoRenameBatchSnapshot {
            source_id: self.source_id.clone(),
            rows: self
                .requested_paths
                .iter()
                .map(|requested_path| AutoRenameBatchRowSnapshot {
                    requested_path: requested_path.clone(),
                    current_path: self
                        .remaps
                        .get(requested_path)
                        .cloned()
                        .unwrap_or_else(|| requested_path.clone()),
                    state: self
                        .states
                        .get(requested_path)
                        .copied()
                        .unwrap_or(AutoRenameBatchRowState::Queued),
                })
                .collect(),
            current_path: self.current_requested_path.as_ref().map(|path| {
                self.remaps
                    .get(path)
                    .cloned()
                    .unwrap_or_else(|| path.clone())
            }),
            remaps: self
                .remaps
                .iter()
                .map(|(old, new)| (old.clone(), new.clone()))
                .collect(),
        }
    }
}
