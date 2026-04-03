use super::*;

impl AppController {
    /// Register one pending async overwrite transaction keyed by a background job.
    pub(crate) fn begin_pending_sample_overwrite_transaction(
        &mut self,
        key: PendingHistoryTransactionKey,
        label: impl Into<String>,
        source_id: SourceId,
        relative_path: PathBuf,
        absolute_path: PathBuf,
    ) -> Result<(), String> {
        if self.history_restoring() {
            return Ok(());
        }
        let before = self.capture_meaningful_ui_snapshot();
        self.history.pending_transactions.insert(
            key,
            PendingHistoryTransaction::SampleOverwrite(PendingSampleOverwriteTransaction {
                label: label.into(),
                before,
                source_id,
                relative_path,
                absolute_path,
            }),
        );
        Ok(())
    }

    /// Register one pending async sample-creation transaction keyed by a background job.
    pub(crate) fn begin_pending_sample_creation_transaction(
        &mut self,
        key: PendingHistoryTransactionKey,
        label: impl Into<String>,
    ) {
        if self.history_restoring() {
            return;
        }
        let before = self.capture_meaningful_ui_snapshot();
        self.history.pending_transactions.insert(
            key,
            PendingHistoryTransaction::SampleCreation(PendingSampleCreationTransaction {
                label: label.into(),
                before,
            }),
        );
    }

    /// Drop one pending async history transaction without creating an undo entry.
    pub(crate) fn cancel_pending_history_transaction(
        &mut self,
        key: &PendingHistoryTransactionKey,
    ) {
        self.history.pending_transactions.remove(key);
    }

    /// Finalize one pending async overwrite transaction after the file job succeeds.
    pub(crate) fn finish_pending_sample_overwrite_transaction(
        &mut self,
        key: &PendingHistoryTransactionKey,
        backup: undo::OverwriteBackup,
    ) -> Result<(), String> {
        let Some(PendingHistoryTransaction::SampleOverwrite(pending)) =
            self.history.pending_transactions.remove(key)
        else {
            return Ok(());
        };
        let after = self.capture_meaningful_ui_snapshot();
        let entry = self.selection_edit_undo_entry(
            pending.label,
            pending.source_id,
            pending.relative_path,
            pending.absolute_path,
            backup,
        );
        self.push_undo_entry(Self::attach_meaningful_ui_restore(
            entry,
            pending.before,
            after,
        ));
        Ok(())
    }

    /// Finalize one pending async sample-creation transaction after the file job succeeds.
    pub(crate) fn finish_pending_sample_creation_transaction(
        &mut self,
        key: &PendingHistoryTransactionKey,
        source_id: SourceId,
        relative_path: PathBuf,
        absolute_path: PathBuf,
        tag: crate::sample_sources::Rating,
        backup: undo::OverwriteBackup,
        label_override: Option<String>,
    ) -> Result<(), String> {
        let Some(PendingHistoryTransaction::SampleCreation(pending)) =
            self.history.pending_transactions.remove(key)
        else {
            return Ok(());
        };
        let after = self.capture_meaningful_ui_snapshot();
        let entry = self.crop_new_sample_undo_entry(
            label_override.unwrap_or(pending.label),
            source_id,
            relative_path,
            absolute_path,
            tag,
            backup,
        );
        self.push_undo_entry(Self::attach_meaningful_ui_restore(
            entry,
            pending.before,
            after,
        ));
        Ok(())
    }

    pub(crate) fn attach_meaningful_ui_restore(
        entry: undo::UndoEntry<AppController>,
        before: MeaningfulUiSnapshot,
        after: MeaningfulUiSnapshot,
    ) -> undo::UndoEntry<AppController> {
        entry
            .with_post_undo(move |controller| {
                controller.restore_meaningful_ui_snapshot(&before);
            })
            .with_post_redo(move |controller| {
                controller.restore_meaningful_ui_snapshot(&after);
            })
    }
}
