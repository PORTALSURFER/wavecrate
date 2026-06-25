use super::*;
use crate::sample_sources::db::SourceWriteBatch;

impl FolderSampleMoveTransaction<'_> {
    /// Commit the target/source DB stages or roll the staged file back on failure.
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::folder_moves::worker) fn commit_db_stage(
        &self,
        errors: &mut Vec<String>,
    ) -> bool {
        let mut batch = match self.db.write_batch() {
            Ok(batch) => batch,
            Err(err) => {
                report_staged_move_failure(
                    errors,
                    self.db,
                    &self.prepared,
                    format!("Failed to start database update: {err}"),
                );
                return false;
            }
        };
        if let Err(err) = batch.remove_file(&self.request.relative_path) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to drop old entry: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.upsert_file(
            &self.request.target_relative,
            self.prepared.file_size,
            self.prepared.modified_ns,
        ) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to register moved file: {err}"),
            );
            return false;
        }
        if !self.copy_metadata_to_target(&mut batch, errors) {
            return false;
        }
        if let Err(err) = batch.commit() {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to save move: {err}"),
            );
            return false;
        }
        self.record_committed_db_stages(errors);
        true
    }

    fn copy_metadata_to_target(
        &self,
        batch: &mut SourceWriteBatch<'_>,
        errors: &mut Vec<String>,
    ) -> bool {
        if let Err(err) = batch.set_tag(&self.request.target_relative, self.metadata.tag) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy tag: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.set_looped(&self.request.target_relative, self.metadata.looped) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy loop marker: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.set_locked(&self.request.target_relative, self.metadata.locked) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy keep lock: {err}"),
            );
            return false;
        }
        if let Some(last_played_at) = self.metadata.last_played_at
            && let Err(err) =
                batch.set_last_played_at(&self.request.target_relative, last_played_at)
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy playback age: {err}"),
            );
            return false;
        }
        if let Some(last_curated_at) = self.metadata.last_curated_at
            && let Err(err) =
                batch.set_last_curated_at(&self.request.target_relative, last_curated_at)
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy curation timestamp: {err}"),
            );
            return false;
        }
        if let Some(sound_type) = self.metadata.sound_type
            && let Err(err) = batch.set_sound_type(&self.request.target_relative, Some(sound_type))
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy sound type: {err}"),
            );
            return false;
        }
        if let Some(user_tag) = self.metadata.user_tag.as_deref()
            && let Err(err) = batch.set_user_tag(&self.request.target_relative, Some(user_tag))
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy custom tag: {err}"),
            );
            return false;
        }
        if let Err(err) =
            batch.replace_tags_for_path(&self.request.target_relative, &self.metadata.normal_tags)
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy normal tags: {err}"),
            );
            return false;
        }
        if let Err(err) =
            batch.set_collection(&self.request.target_relative, self.metadata.collection)
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy collection: {err}"),
            );
            return false;
        }
        true
    }
}
