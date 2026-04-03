use super::super::*;
use std::fs;
use std::path::PathBuf;

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
        let normalized = crate::sample_sources::config::normalize_path(path.as_path());
        if !normalized.is_dir() {
            return Err("Please select a directory".into());
        }
        if self.library.sources.iter().any(|s| s.root == normalized) {
            self.set_status("Source already added", StatusTone::Info);
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
        SourceDatabase::open(&normalized)
            .map_err(|err| format!("Failed to create database: {err}"))?;
        let _ = self.cache_db(&source);
        self.library.sources.push(source.clone());
        self.refresh_source_watcher();
        self.select_source(Some(source.id.clone()));
        self.persist_config("Failed to save config after adding source")?;
        self.prepare_similarity_for_selected_source();
        Ok(())
    }

    /// Remove a configured source by index.
    pub fn remove_source(&mut self, index: usize) {
        if index >= self.library.sources.len() {
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
        let Some(existing) = self.library.sources.get(index) else {
            return Err("Source not found".into());
        };
        let normalized = crate::sample_sources::config::normalize_path(new_root.as_path());
        if !normalized.is_dir() {
            return Err("Please select a directory".into());
        }
        if self
            .library
            .sources
            .iter()
            .enumerate()
            .any(|(i, source)| i != index && source.root == normalized)
        {
            return Err("Source already added".into());
        }
        let old_db_path = crate::sample_sources::database_path_for(&existing.root);
        let new_db_path = crate::sample_sources::database_path_for(&normalized);
        if old_db_path.exists() && !new_db_path.exists() {
            let _ = fs::create_dir_all(&normalized);
            fs::copy(&old_db_path, &new_db_path)
                .map_err(|err| format!("Failed to copy database: {err}"))?;
        }
        SourceDatabase::open(&normalized)
            .map_err(|err| format!("Failed to prepare database: {err}"))?;
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
        self.persist_config("Failed to save config after remapping source")?;
        self.refresh_sources_ui();
        self.queue_wav_load();
        self.set_status("Source remapped", StatusTone::Info);
        Ok(())
    }

    /// Open the source root in the OS file explorer.
    pub fn open_source_folder(&mut self, index: usize) {
        let Some(source) = self.library.sources.get(index) else {
            self.set_status("Source not found", StatusTone::Error);
            return;
        };
        if !source.root.exists() {
            self.set_status(
                format!("Source folder missing: {}", source.root.display()),
                StatusTone::Warning,
            );
            return;
        }
        if let Err(err) = open::that(&source.root) {
            self.set_status(
                format!("Could not open folder {}: {err}", source.root.display()),
                StatusTone::Error,
            );
        }
    }
}
