use super::telemetry::record_source_lifecycle_event;
use super::validation::{nested_source_conflict_error, source_roots_match};
use super::*;

struct RemappedSource {
    index: usize,
    root: PathBuf,
    previous_root: PathBuf,
    id: SourceId,
    started_at: Instant,
}

impl AppController {
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
        let Some(existing_source) = self.library.sources.get(index) else {
            let error = String::from("Source not found");
            record_source_lifecycle_event("sources.remap", None, "error", started_at, Some(&error));
            return Err(error);
        };
        let source_id = existing_source.id.clone();
        let normalized = crate::sample_sources::config::normalize_path(new_root.as_path());
        if let Err(error) = validate_remap_source_root(&self.library.sources, index, &normalized) {
            record_source_lifecycle_event(
                "sources.remap",
                Some(source_id.as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return Err(error);
        }
        let previous_root = self.library.sources[index].root.clone();
        self.commit_remapped_source(RemappedSource {
            index,
            root: normalized,
            previous_root,
            id: source_id,
            started_at,
        })
    }

    fn commit_remapped_source(&mut self, remap: RemappedSource) -> Result<(), String> {
        let previous_source = self.library.sources[remap.index].clone();
        let artifacts = RemapArtifactSnapshot::new(&remap.root);
        let prepare_result =
            copy_source_database_if_needed(&previous_source, &remap.root, remap.started_at)
                .and_then(|()| {
                    prepare_database_for_remap(&previous_source, &remap.root, remap.started_at)
                });
        if let Err(error) = prepare_result {
            artifacts.restore_and_remove_created();
            record_source_lifecycle_event(
                "sources.remap",
                Some(remap.id.as_str()),
                "error",
                remap.started_at,
                Some(&error),
            );
            return Err(error);
        }
        self.library.sources[remap.index].root = remap.root.clone();
        if let Err(error) = self.persist_config("Failed to save config after remapping source") {
            self.library.sources[remap.index].root = remap.previous_root;
            artifacts.restore_and_remove_created();
            record_source_lifecycle_event(
                "sources.remap",
                Some(remap.id.as_str()),
                "error",
                remap.started_at,
                Some(&error),
            );
            return Err(error);
        }
        self.library.missing.sources.remove(&remap.id);
        let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
            &mut self.cache,
            &mut self.ui_cache,
            &mut self.library.missing,
        );
        invalidator.invalidate_db_cache(&remap.id);
        invalidator.invalidate_wav_related(&remap.id);
        if self.selection_state.ctx.selected_source.as_ref() == Some(&remap.id) {
            self.clear_wavs();
            self.selection_state.ctx.selected_source = Some(remap.id.clone());
        }
        self.refresh_sources_ui();
        self.queue_wav_load();
        self.set_status("Source remapped", StatusTone::Info);
        record_source_lifecycle_event(
            "sources.remap",
            Some(remap.id.as_str()),
            "success",
            remap.started_at,
            None,
        );
        Ok(())
    }
}

struct RemapArtifactSnapshot {
    paths: Vec<(PathBuf, bool)>,
    legacy_migrations: Vec<(PathBuf, PathBuf, bool, bool)>,
}

impl RemapArtifactSnapshot {
    fn new(root: &Path) -> Self {
        let database = crate::sample_sources::database_path_for(root);
        let legacy_database = root.join(crate::sample_sources::db::LEGACY_DB_FILE_NAME);
        let paths = [
            database.clone(),
            path_with_suffix(&database, "-wal"),
            path_with_suffix(&database, "-shm"),
            path_with_suffix(&database, "-journal"),
        ]
        .into_iter()
        .map(|path| {
            let existed = path.exists();
            (path, existed)
        })
        .collect();
        let legacy_migrations = ["", "-wal", "-shm"]
            .into_iter()
            .map(|suffix| {
                let legacy = path_with_suffix(&legacy_database, suffix);
                let current = path_with_suffix(&database, suffix);
                let legacy_existed = legacy.exists();
                let current_existed = current.exists();
                (legacy, current, legacy_existed, current_existed)
            })
            .collect();
        Self {
            paths,
            legacy_migrations,
        }
    }

    fn restore_and_remove_created(&self) {
        for (legacy, current, legacy_existed, current_existed) in &self.legacy_migrations {
            if *legacy_existed && !*current_existed && !legacy.exists() && current.exists() {
                if let Err(err) = fs::rename(current, legacy) {
                    tracing::warn!(
                        from = %current.display(),
                        to = %legacy.display(),
                        error = %err,
                        "Failed to restore legacy source database artifact after rollback"
                    );
                }
            }
        }
        for (path, existed) in &self.paths {
            if !existed {
                match fs::remove_file(path) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => tracing::warn!(
                        path = %path.display(),
                        error = %err,
                        "Failed to remove remap database artifact after rollback"
                    ),
                }
            }
        }
    }
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn validate_remap_source_root(
    sources: &[SampleSource],
    index: usize,
    normalized: &PathBuf,
) -> Result<(), String> {
    if !normalized.is_dir() {
        return Err(String::from("Please select a directory"));
    }
    if sources
        .iter()
        .enumerate()
        .any(|(i, source)| i != index && source_roots_match(&source.root, normalized))
    {
        return Err(String::from("Source already added"));
    }
    if let Some(error) = sources
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .find_map(|(_, source)| nested_source_conflict_error(&source.root, normalized))
    {
        return Err(error);
    }
    Ok(())
}

fn prepare_database_for_remap(
    existing: &SampleSource,
    normalized: &PathBuf,
    started_at: Instant,
) -> Result<(), String> {
    if let Err(err) = SourceDatabase::open(normalized) {
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
    Ok(())
}

fn copy_source_database_if_needed(
    existing: &SampleSource,
    normalized: &PathBuf,
    started_at: Instant,
) -> Result<(), String> {
    let old_db_path = crate::sample_sources::database_path_for(&existing.root);
    let new_db_path = crate::sample_sources::database_path_for(normalized);
    if !old_db_path.exists() || new_db_path.exists() {
        return Ok(());
    }
    let _ = fs::create_dir_all(normalized);
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
    Ok(())
}
