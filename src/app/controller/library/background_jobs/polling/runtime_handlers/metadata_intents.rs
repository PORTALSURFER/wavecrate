use super::*;
use crate::app::controller::state::runtime::MetadataRollback;

impl AppController {
    pub(in crate::app::controller::library::background_jobs::polling::runtime_handlers) fn finish_metadata_mutation_intents(
        &mut self,
        source_id: &SourceId,
        rollback: &[MetadataRollback],
    ) {
        for entry in rollback {
            if let MetadataRollback::Looped {
                relative_path,
                intent_id,
                ..
            } = entry
            {
                let relative_path =
                    self.resolve_looped_rollback_path(source_id, relative_path, *intent_id);
                self.runtime
                    .source_lane
                    .mutations
                    .finish_looped_metadata_intent(source_id, &relative_path, *intent_id);
            }
        }
    }

    pub(in crate::app::controller::library::background_jobs::polling::runtime_handlers) fn resolve_looped_rollback_path(
        &self,
        source_id: &SourceId,
        relative_path: &std::path::Path,
        intent_id: u64,
    ) -> std::path::PathBuf {
        if self
            .runtime
            .source_lane
            .mutations
            .looped_metadata_intent_matches(source_id, relative_path, intent_id)
        {
            return relative_path.to_path_buf();
        }
        if let Some(new_relative) =
            crate::app::controller::library::source_write_priority::completed_browser_rename_target(
                source_id,
                relative_path,
            )
            && self
                .runtime
                .source_lane
                .mutations
                .looped_metadata_intent_matches(source_id, &new_relative, intent_id)
        {
            return new_relative;
        }
        relative_path.to_path_buf()
    }
}
