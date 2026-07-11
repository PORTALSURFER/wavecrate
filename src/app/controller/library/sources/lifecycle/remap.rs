use super::telemetry::record_source_lifecycle_event;
use super::validation::{nested_source_conflict_error, source_roots_match};
use super::*;
#[cfg(not(test))]
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::jobs::{SourceRemapJob, SourceRemapPreparedResult};
#[cfg(not(test))]
use crate::app::controller::state::runtime::PendingSourceRemap;

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
        if self.runtime.source_lane.pending_remap.is_some() {
            return Err(String::from("Source remap already in progress"));
        }
        let job = SourceRemapJob {
            request_id: self.runtime.jobs.next_source_remap_request_id(),
            source: existing_source.clone(),
            new_root: normalized,
        };
        #[cfg(test)]
        {
            let prepared = run_source_remap_prepare(job);
            prepared.result?;
            let result = self.commit_remapped_source(RemappedSource {
                index,
                root: prepared.new_root.clone(),
                previous_root: existing_source.root.clone(),
                id: source_id,
                started_at,
            });
            if result.is_err() {
                remove_database_artifacts_if_created(
                    &prepared.new_root,
                    prepared.artifacts_created,
                );
            }
            result
        }
        #[cfg(not(test))]
        {
            self.queue_source_remap_prepare(job, started_at);
            Ok(())
        }
    }

    #[cfg(not(test))]
    fn queue_source_remap_prepare(&mut self, job: SourceRemapJob, started_at: Instant) {
        self.runtime.source_lane.pending_remap = Some(PendingSourceRemap {
            request_id: job.request_id,
            source: job.source.clone(),
            new_root: job.new_root.clone(),
            queued_at: started_at,
            canceled: false,
        });
        self.set_status(
            format!("Remapping source to {}", job.new_root.display()),
            StatusTone::Info,
        );
        self.runtime.jobs.spawn_one_shot_job(
            true,
            move || run_source_remap_prepare(job),
            JobMessage::SourceRemapPrepared,
        );
    }

    pub(crate) fn handle_source_remap_prepared_message(
        &mut self,
        message: SourceRemapPreparedResult,
    ) {
        let is_current = self
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| {
                pending.request_id == message.request_id
                    && pending.source.id == message.source.id
                    && pending.new_root == message.new_root
            });
        if !is_current {
            return;
        }
        let pending = self
            .runtime
            .source_lane
            .pending_remap
            .take()
            .expect("current source remap pending entry");
        if pending.canceled {
            remove_database_artifacts_if_created(&message.new_root, message.artifacts_created);
            return;
        }
        let Some(index) = self.library.sources.iter().position(|source| {
            source.id == message.source.id && source.root == message.source.root
        }) else {
            remove_database_artifacts_if_created(&message.new_root, message.artifacts_created);
            return;
        };
        match message.result {
            Ok(()) => {
                if let Err(error) =
                    validate_remap_source_root(&self.library.sources, index, &message.new_root)
                        .and_then(|()| {
                            self.commit_remapped_source(RemappedSource {
                                index,
                                root: message.new_root.clone(),
                                previous_root: message.source.root.clone(),
                                id: message.source.id.clone(),
                                started_at: pending.queued_at,
                            })
                        })
                {
                    remove_database_artifacts_if_created(
                        &message.new_root,
                        message.artifacts_created,
                    );
                    self.set_status(error, StatusTone::Error);
                }
            }
            Err(error) => self.set_status(error, StatusTone::Error),
        }
    }

    fn commit_remapped_source(&mut self, remap: RemappedSource) -> Result<(), String> {
        self.library.sources[remap.index].root = remap.root;
        if let Err(err) = self.persist_config("Failed to save config after remapping source") {
            self.library.sources[remap.index].root = remap.previous_root;
            record_source_lifecycle_event(
                "sources.remap",
                Some(remap.id.as_str()),
                "error",
                remap.started_at,
                Some(&err),
            );
            return Err(err);
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

fn run_source_remap_prepare(job: SourceRemapJob) -> SourceRemapPreparedResult {
    let destination = crate::sample_sources::database_path_for(&job.new_root);
    let destination_existed = database_artifact_present(&destination);
    let mut artifacts_created = false;
    let result = (|| {
        let source_database = crate::sample_sources::database_path_for(&job.source.root);
        if !destination_existed {
            if source_database.exists() {
                let source = SourceDatabase::open(&job.source.root).map_err(|error| {
                    format!("Failed to open source database for snapshot: {error}")
                })?;
                source
                    .snapshot_to_path(&destination)
                    .map_err(|error| format!("Failed to snapshot source database: {error}"))?;
            } else {
                reserve_empty_database_path(&destination)?;
            }
            artifacts_created = true;
        }
        SourceDatabase::open(&job.new_root)
            .map(|_| ())
            .map_err(|error| format!("Failed to prepare database: {error}"))
    })();
    if result.is_err() {
        remove_database_artifacts_if_created(&job.new_root, artifacts_created);
    }
    SourceRemapPreparedResult {
        request_id: job.request_id,
        source: job.source,
        new_root: job.new_root,
        artifacts_created,
        result,
    }
}

fn remove_database_artifacts_if_created(root: &Path, artifacts_created: bool) {
    if !artifacts_created {
        return;
    }
    let database = crate::sample_sources::database_path_for(root);
    for path in [
        database.clone(),
        path_with_suffix(&database, "-wal"),
        path_with_suffix(&database, "-shm"),
        path_with_suffix(&database, "-journal"),
    ] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(error = %error, "Failed to remove remap snapshot artifact")
            }
        }
    }
}

fn database_artifact_present(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

fn reserve_empty_database_path(path: &Path) -> Result<(), String> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map(drop)
        .map_err(|error| format!("Failed to reserve destination database: {error}"))
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}
