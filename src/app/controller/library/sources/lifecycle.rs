use super::super::*;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::logging::{ActionDebugEvent, emit_action_debug_event};

impl AppController {
    /// Add a new source folder via file picker.
    pub fn add_source_via_dialog(&mut self) {
        let Some(path) = FileDialog::new().pick_folder() else {
            return;
        };
        if let Err(error) = self.add_source_from_path(path) {
            self.set_status(error, StatusTone::Error);
        }
    }

    /// Add a new source folder from a known path.
    pub fn add_source_from_path(&mut self, path: PathBuf) -> Result<(), String> {
        let started_at = Instant::now();
        let normalized = crate::sample_sources::config::normalize_path(path.as_path());
        let source = normalized.display().to_string();
        if !normalized.is_dir() {
            let error = String::from("Please select a directory");
            record_source_lifecycle_event(
                "sources.add",
                Some(&source),
                "error",
                started_at,
                Some(&error),
            );
            return Err(error);
        }
        if self.library.sources.iter().any(|s| s.root == normalized) {
            self.set_status("Source already added", StatusTone::Info);
            record_source_lifecycle_event(
                "sources.add",
                Some(&source),
                "short_circuit",
                started_at,
                Some("already_added"),
            );
            return Ok(());
        }
        let source = match crate::sample_sources::library::lookup_source_id_for_root(&normalized) {
            Ok(Some(id)) => SampleSource::new_with_id(id, normalized.clone()),
            Ok(None) => SampleSource::new(normalized.clone()),
            Err(err) => {
                self.set_status(
                    format!("Could not check library history (continuing): {err}"),
                    StatusTone::Warning,
                );
                SampleSource::new(normalized.clone())
            }
        };
        if let Err(err) = SourceDatabase::open(&normalized) {
            let error = format!("Failed to create database: {err}");
            record_source_lifecycle_event(
                "sources.add",
                Some(source.id.as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return Err(error);
        }
        let _ = self.cache_db(&source);
        self.library.sources.push(source.clone());
        self.refresh_source_watcher();
        self.select_source(Some(source.id.clone()));
        if let Err(err) = self.persist_config("Failed to save config after adding source") {
            record_source_lifecycle_event(
                "sources.add",
                Some(source.id.as_str()),
                "error",
                started_at,
                Some(&err),
            );
            return Err(err);
        }
        self.prepare_similarity_for_selected_source();
        record_source_lifecycle_event(
            "sources.add",
            Some(source.id.as_str()),
            "success",
            started_at,
            None,
        );
        Ok(())
    }

    /// Remove a configured source by index.
    pub fn remove_source(&mut self, index: usize) {
        let started_at = Instant::now();
        if index >= self.library.sources.len() {
            record_source_lifecycle_event(
                "sources.remove",
                None,
                "short_circuit",
                started_at,
                Some("source_not_found"),
            );
            return;
        }
        let removed = self.library.sources.remove(index);
        self.library.missing.sources.remove(&removed.id);
        for pane in [FolderPaneId::Upper, FolderPaneId::Lower] {
            if self.ui.sources.folder_pane(pane).source_id.as_ref() == Some(&removed.id) {
                let pane_state = self.ui.sources.folder_pane_mut(pane);
                pane_state.source_id = None;
                pane_state.browser = FolderBrowserUiState::default();
            }
        }
        let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
            &mut self.cache,
            &mut self.ui_cache,
            &mut self.library.missing,
        );
        invalidator.invalidate_all(&removed.id);
        if self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .is_some_and(|id| id == &removed.id)
        {
            self.selection_state.ctx.selected_source = None;
            self.sample_view.wav.selected_wav = None;
            self.clear_focused_similarity_highlight();
            self.clear_waveform_view();
        }
        let _ = self.persist_config("Failed to save config after removing source");
        self.refresh_source_watcher();
        self.refresh_sources_ui();
        let _ = self.refresh_wavs();
        self.select_first_source();
        self.set_status("Source removed", StatusTone::Info);
        record_source_lifecycle_event(
            "sources.remove",
            Some(removed.id.as_str()),
            "success",
            started_at,
            None,
        );
    }

    pub(crate) fn database_for(
        &mut self,
        source: &SampleSource,
    ) -> Result<Rc<SourceDatabase>, SourceDbError> {
        self.cache.database_for(source)
    }

    pub(crate) fn cache_db(
        &mut self,
        source: &SampleSource,
    ) -> Result<Rc<SourceDatabase>, SourceDbError> {
        self.database_for(source)
    }

    /// Remap a source root via folder picker.
    pub fn remap_source_via_dialog(&mut self, index: usize) {
        let Some(path) = FileDialog::new().pick_folder() else {
            return;
        };
        if let Err(error) = self.remap_source_to(index, path) {
            self.set_status(error, StatusTone::Error);
        }
    }

    /// Remap a source to a new root path, preserving the source id and tags.
    pub fn remap_source_to(&mut self, index: usize, new_root: PathBuf) -> Result<(), String> {
        let started_at = Instant::now();
        let Some(existing) = self.library.sources.get(index) else {
            let error = String::from("Source not found");
            record_source_lifecycle_event("sources.remap", None, "error", started_at, Some(&error));
            return Err(error);
        };
        let normalized = crate::sample_sources::config::normalize_path(new_root.as_path());
        if !normalized.is_dir() {
            let error = String::from("Please select a directory");
            record_source_lifecycle_event(
                "sources.remap",
                Some(existing.id.as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return Err(error);
        }
        if self
            .library
            .sources
            .iter()
            .enumerate()
            .any(|(i, source)| i != index && source.root == normalized)
        {
            let error = String::from("Source already added");
            record_source_lifecycle_event(
                "sources.remap",
                Some(existing.id.as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return Err(error);
        }
        let old_db_path = crate::sample_sources::database_path_for(&existing.root);
        let new_db_path = crate::sample_sources::database_path_for(&normalized);
        if old_db_path.exists() && !new_db_path.exists() {
            let _ = fs::create_dir_all(&normalized);
            fs::copy(&old_db_path, &new_db_path).map_err(|err| {
                let error = format!("Failed to copy database: {err}");
                record_source_lifecycle_event(
                    "sources.remap",
                    Some(existing.id.as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
                error
            })?;
        }
        if let Err(err) = SourceDatabase::open(&normalized) {
            let error = format!("Failed to prepare database: {err}");
            record_source_lifecycle_event(
                "sources.remap",
                Some(existing.id.as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return Err(error);
        }
        let source_id = existing.id.clone();
        self.library.sources[index].root = normalized.clone();
        self.library.missing.sources.remove(&source_id);
        let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
            &mut self.cache,
            &mut self.ui_cache,
            &mut self.library.missing,
        );
        invalidator.invalidate_db_cache(&source_id);
        invalidator.invalidate_wav_related(&source_id);
        if self.selection_state.ctx.selected_source.as_ref() == Some(&source_id) {
            self.clear_wavs();
            self.selection_state.ctx.selected_source = Some(source_id.clone());
        }
        if let Err(err) = self.persist_config("Failed to save config after remapping source") {
            record_source_lifecycle_event(
                "sources.remap",
                Some(source_id.as_str()),
                "error",
                started_at,
                Some(&err),
            );
            return Err(err);
        }
        self.refresh_sources_ui();
        self.queue_wav_load();
        self.set_status("Source remapped", StatusTone::Info);
        record_source_lifecycle_event(
            "sources.remap",
            Some(source_id.as_str()),
            "success",
            started_at,
            None,
        );
        Ok(())
    }

    /// Open the source root in the OS file explorer.
    pub fn open_source_folder(&mut self, index: usize) {
        let started_at = Instant::now();
        let Some(source) = self.library.sources.get(index) else {
            self.set_status("Source not found", StatusTone::Error);
            record_source_lifecycle_event(
                "sources.open_folder",
                None,
                "error",
                started_at,
                Some("source_not_found"),
            );
            return;
        };
        let source_id = source.id.as_str().to_string();
        let source_root = source.root.clone();
        if !source_root.exists() {
            self.set_status(
                format!("Source folder missing: {}", source_root.display()),
                StatusTone::Warning,
            );
            record_source_lifecycle_event(
                "sources.open_folder",
                Some(&source_id),
                "error",
                started_at,
                Some("source_root_missing"),
            );
            return;
        }
        if let Err(err) = open::that(&source_root) {
            self.set_status(
                format!("Could not open folder {}: {err}", source_root.display()),
                StatusTone::Error,
            );
            let error = err.to_string();
            record_source_lifecycle_event(
                "sources.open_folder",
                Some(&source_id),
                "error",
                started_at,
                Some(&error),
            );
            return;
        }
        record_source_lifecycle_event(
            "sources.open_folder",
            Some(&source_id),
            "success",
            started_at,
            None,
        );
    }
}

fn record_source_lifecycle_event(
    action: &'static str,
    source: Option<&str>,
    outcome: &'static str,
    started_at: Instant,
    error: Option<&str>,
) {
    emit_action_debug_event(ActionDebugEvent {
        action,
        pane: Some("sources"),
        source,
        outcome,
        elapsed: started_at.elapsed(),
        error,
    });
}
