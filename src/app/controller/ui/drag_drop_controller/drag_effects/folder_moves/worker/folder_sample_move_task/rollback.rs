use super::*;
use crate::sample_sources::db::SourceWriteBatch;

impl FolderSampleMoveTransaction<'_> {
    /// Restore the original DB row and staged file after the DB commit already succeeded.
    pub(super) fn rollback_after_finalize_failure(
        &self,
        errors: &mut Vec<String>,
        message: String,
    ) {
        errors.push(message);
        let db_restored = self.rollback_db_stage(errors);
        let file_restored = match move_sample_file(
            &self.prepared.staged_absolute,
            &self.prepared.source_absolute,
        ) {
            Ok(()) => true,
            Err(err) => {
                errors.push(format!("Failed to restore moved file: {err}"));
                false
            }
        };
        if db_restored && file_restored {
            self.clear_move_journal_entry(errors);
        } else {
            errors.push("Move left staged for recovery".to_string());
        }
    }

    /// Roll the committed target/source DB stages back to the original source row.
    fn rollback_db_stage(&self, errors: &mut Vec<String>) -> bool {
        let mut batch = match self.db.write_batch() {
            Ok(batch) => batch,
            Err(err) => {
                errors.push(format!("Failed to start database rollback: {err}"));
                return false;
            }
        };
        if let Err(err) = batch.remove_file(&self.request.target_relative) {
            errors.push(format!("Failed to remove rolled-back target entry: {err}"));
            return false;
        }
        if let Err(err) = batch.upsert_file(
            &self.request.relative_path,
            self.prepared.file_size,
            self.prepared.modified_ns,
        ) {
            errors.push(format!("Failed to restore original database entry: {err}"));
            return false;
        }
        if !self.restore_source_metadata(&mut batch, errors) {
            return false;
        }
        if let Err(err) = batch.commit() {
            errors.push(format!("Failed to commit database rollback: {err}"));
            return false;
        }
        true
    }

    fn restore_source_metadata(
        &self,
        batch: &mut SourceWriteBatch<'_>,
        errors: &mut Vec<String>,
    ) -> bool {
        if let Err(err) = batch.set_tag(&self.request.relative_path, self.metadata.tag) {
            errors.push(format!("Failed to restore tag: {err}"));
            return false;
        }
        if let Err(err) = batch.set_looped(&self.request.relative_path, self.metadata.looped) {
            errors.push(format!("Failed to restore loop marker: {err}"));
            return false;
        }
        if let Err(err) = batch.set_locked(&self.request.relative_path, self.metadata.locked) {
            errors.push(format!("Failed to restore keep lock: {err}"));
            return false;
        }
        if let Some(last_played_at) = self.metadata.last_played_at
            && let Err(err) = batch.set_last_played_at(&self.request.relative_path, last_played_at)
        {
            errors.push(format!("Failed to restore playback age: {err}"));
            return false;
        }
        if let Err(err) =
            batch.set_sound_type(&self.request.relative_path, self.metadata.sound_type)
        {
            errors.push(format!("Failed to restore sound type: {err}"));
            return false;
        }
        if let Err(err) = batch.set_user_tag(
            &self.request.relative_path,
            self.metadata.user_tag.as_deref(),
        ) {
            errors.push(format!("Failed to restore custom tag: {err}"));
            return false;
        }
        if let Err(err) =
            batch.replace_tags_for_path(&self.request.relative_path, &self.metadata.normal_tags)
        {
            errors.push(format!("Failed to restore normal tags: {err}"));
            return false;
        }
        if let Err(err) =
            batch.set_collection(&self.request.relative_path, self.metadata.collection)
        {
            errors.push(format!("Failed to restore collection: {err}"));
            return false;
        }
        true
    }
}
