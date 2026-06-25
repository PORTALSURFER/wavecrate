use super::*;

impl FolderSampleMoveTransaction<'_> {
    /// Clear the journal entry and build the success payload for the moved sample.
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::folder_moves::worker) fn into_success(
        self,
        errors: &mut Vec<String>,
    ) -> FolderEntryMove {
        self.clear_move_journal_entry(errors);
        FolderEntryMove {
            old_relative: self.request.relative_path,
            new_relative: self.request.target_relative,
            file_size: self.prepared.file_size,
            modified_ns: self.prepared.modified_ns,
            tag: self.metadata.tag,
            looped: self.metadata.looped,
            locked: self.metadata.locked,
            last_played_at: self.metadata.last_played_at,
            last_curated_at: self.metadata.last_curated_at,
            sound_type: self.metadata.sound_type,
            user_tag: self.metadata.user_tag,
            normal_tags: self.metadata.normal_tags,
            collection: self.metadata.collection,
        }
    }
}
