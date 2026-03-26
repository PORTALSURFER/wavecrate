use crate::app::controller::AppController;
use crate::sample_sources::{Rating, WavEntry};
use std::path::Path;

/// Sample metadata persisted when registering a newly moved or copied file.
pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) struct MovedSampleRegistration
{
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) file_size: u64,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) modified_ns: i64,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) tag: Rating,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) looped: bool,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) locked: bool,
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) last_played_at:
        Option<i64>,
}

impl AppController {
    /// Register a moved sample in a destination source database with preserved metadata.
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) fn register_moved_sample_for_source(
        &mut self,
        source: &crate::sample_sources::SampleSource,
        relative_path: &Path,
        registration: MovedSampleRegistration,
    ) -> Result<(), String> {
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        db.upsert_file(
            relative_path,
            registration.file_size,
            registration.modified_ns,
        )
        .map_err(|err| format!("Failed to register file: {err}"))?;
        db.set_tag(relative_path, registration.tag)
            .map_err(|err| format!("Failed to set tag: {err}"))?;
        db.set_looped(relative_path, registration.looped)
            .map_err(|err| format!("Failed to set loop marker: {err}"))?;
        db.set_locked(relative_path, registration.locked)
            .map_err(|err| format!("Failed to set keep lock: {err}"))?;
        if let Some(last_played_at) = registration.last_played_at {
            db.set_last_played_at(relative_path, last_played_at)
                .map_err(|err| format!("Failed to copy playback age: {err}"))?;
        }
        Ok(())
    }

    /// Remove a moved sample's original row from its source database.
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects) fn remove_source_db_entry(
        &mut self,
        source: &crate::sample_sources::SampleSource,
        relative_path: &Path,
    ) -> Result<(), String> {
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        db.remove_file(relative_path)
            .map_err(|err| format!("Failed to drop database row: {err}"))
    }

    /// Insert one moved target entry into the in-memory cache for the destination source.
    pub(super) fn insert_moved_target_entry(
        &mut self,
        target_source: &crate::sample_sources::SampleSource,
        entry: &crate::app::controller::jobs::SourceMoveSuccess,
    ) {
        self.insert_cached_entry(
            target_source,
            WavEntry {
                relative_path: entry.target_relative.clone(),
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash: None,
                tag: entry.tag,
                looped: entry.looped,
                locked: entry.locked,
                missing: false,
                last_played_at: entry.last_played_at,
            },
        );
    }
}
