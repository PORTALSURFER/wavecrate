use super::telemetry::record_source_lifecycle_event;
use super::validation::{nested_source_conflict_error, source_roots_match};
use super::*;
#[cfg(not(test))]
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::jobs::{SourceAddJob, SourceAddPreparedResult};
use crate::app::controller::state::runtime::PendingSourceAdd;
use crate::app::state::ProgressTaskKind;

#[cfg(test)]
use std::{cell::Cell, thread_local};

enum AddSourceRoot {
    New(PathBuf),
    AlreadyAdded(String),
}

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
        let normalized = match validate_add_source_root(&self.library.sources, path) {
            Ok(AddSourceRoot::New(root)) => root,
            Ok(AddSourceRoot::AlreadyAdded(source)) => {
                record_source_lifecycle_event(
                    "sources.add",
                    Some(&source),
                    "short_circuit",
                    started_at,
                    Some("already_added"),
                );
                self.set_status("Source already added", StatusTone::Info);
                return Ok(());
            }
            Err(error) => {
                record_source_lifecycle_event(
                    "sources.add",
                    Some(error.source.as_str()),
                    "error",
                    started_at,
                    Some(error.message.as_str()),
                );
                return Err(error.message);
            }
        };
        let source = self.source_for_add_root(&normalized);
        if let Some(message) = self.pending_source_add_conflict(&normalized) {
            self.set_status(message, StatusTone::Info);
            return Ok(());
        }
        if source_add_async_enabled() {
            self.queue_source_add_prepare(source, started_at);
            return Ok(());
        }
        let result = run_source_add_prepare(SourceAddJob {
            request_id: self.runtime.jobs.next_source_add_request_id(),
            source,
        });
        match result.result {
            Ok(()) => self.commit_added_source(result.source, started_at),
            Err(err) => Err(err),
        }
    }

    fn source_for_add_root(&mut self, normalized: &Path) -> SampleSource {
        match crate::sample_sources::library::lookup_source_id_for_root(normalized) {
            Ok(Some(id)) => SampleSource::new_with_id(id, normalized.to_path_buf()),
            Ok(None) => SampleSource::new(normalized.to_path_buf()),
            Err(err) => {
                self.set_status(
                    format!("Could not check library history (continuing): {err}"),
                    StatusTone::Warning,
                );
                SampleSource::new(normalized.to_path_buf())
            }
        }
    }

    fn commit_added_source(
        &mut self,
        source: SampleSource,
        started_at: Instant,
    ) -> Result<(), String> {
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

    fn queue_source_add_prepare(&mut self, source: SampleSource, started_at: Instant) {
        let request_id = self.runtime.jobs.next_source_add_request_id();
        self.runtime.source_lane.pending_adds.insert(
            source.root.clone(),
            PendingSourceAdd {
                request_id,
                source: source.clone(),
                queued_at: started_at,
            },
        );
        self.show_status_progress(ProgressTaskKind::SourceAdd, "Adding source", 1, false);
        self.update_progress_detail_for_task(
            ProgressTaskKind::SourceAdd,
            source.root.display().to_string(),
        );
        self.set_status(
            format!("Adding source {}", source.root.display()),
            StatusTone::Info,
        );
        record_source_lifecycle_event(
            "sources.add",
            Some(source.id.as_str()),
            "prepare_queued",
            started_at,
            None,
        );

        let job = SourceAddJob { request_id, source };
        #[cfg(not(test))]
        self.runtime.jobs.spawn_one_shot_job(
            true,
            move || run_source_add_prepare(job),
            JobMessage::SourceAddPrepared,
        );
        #[cfg(test)]
        {
            let _ = job;
        }
    }

    pub(crate) fn handle_source_add_prepared_message(&mut self, message: SourceAddPreparedResult) {
        let is_current = self
            .runtime
            .source_lane
            .pending_adds
            .get(&message.source.root)
            .is_some_and(|pending| {
                pending.request_id == message.request_id && pending.source.id == message.source.id
            });
        if !is_current {
            return;
        };
        let pending = self
            .runtime
            .source_lane
            .pending_adds
            .remove(&message.source.root)
            .expect("current source add pending entry");
        self.clear_progress_task(ProgressTaskKind::SourceAdd);
        let started_at = pending.queued_at;
        match message.result {
            Ok(()) => {
                if self
                    .library
                    .sources
                    .iter()
                    .any(|source| source_roots_match(&source.root, &message.source.root))
                {
                    record_source_lifecycle_event(
                        "sources.add",
                        Some(message.source.id.as_str()),
                        "short_circuit",
                        started_at,
                        Some("already_added_after_prepare"),
                    );
                    self.set_status("Source already added", StatusTone::Info);
                    return;
                }
                if let Err(err) = self.commit_added_source(message.source, started_at) {
                    self.set_status(err, StatusTone::Error);
                }
            }
            Err(err) => {
                record_source_lifecycle_event(
                    "sources.add",
                    Some(message.source.id.as_str()),
                    "error",
                    started_at,
                    Some(err.as_str()),
                );
                self.set_status(err, StatusTone::Error);
            }
        }
    }

    fn pending_source_add_conflict(&self, normalized: &PathBuf) -> Option<String> {
        for pending in self.runtime.source_lane.pending_adds.values() {
            if source_roots_match(&pending.source.root, normalized) {
                return Some(String::from("Source add already in progress"));
            }
            if let Some(message) = nested_source_conflict_error(&pending.source.root, normalized) {
                return Some(message);
            }
            if let Some(message) = nested_source_conflict_error(normalized, &pending.source.root) {
                return Some(message);
            }
        }
        None
    }
}

struct AddSourceRootError {
    source: String,
    message: String,
}

fn validate_add_source_root(
    sources: &[SampleSource],
    path: PathBuf,
) -> Result<AddSourceRoot, AddSourceRootError> {
    let normalized = crate::sample_sources::config::normalize_path(path.as_path());
    let source = normalized.display().to_string();
    if !normalized.is_dir() {
        return Err(AddSourceRootError {
            source,
            message: String::from("Please select a directory"),
        });
    }
    if sources
        .iter()
        .any(|s| source_roots_match(&s.root, &normalized))
    {
        return Ok(AddSourceRoot::AlreadyAdded(source));
    }
    if let Some(message) = sources
        .iter()
        .find_map(|s| nested_source_conflict_error(&s.root, &normalized))
    {
        return Err(AddSourceRootError { source, message });
    }
    Ok(AddSourceRoot::New(normalized))
}

fn run_source_add_prepare(job: SourceAddJob) -> SourceAddPreparedResult {
    let started_at = Instant::now();
    let result = SourceDatabase::open_for_source_write(&job.source.root)
        .map(|_| ())
        .map_err(|err| {
            let error = format!("Failed to create database: {err}");
            record_source_lifecycle_event(
                "sources.add",
                Some(job.source.id.as_str()),
                "error",
                started_at,
                Some(&error),
            );
            error
        });
    SourceAddPreparedResult {
        request_id: job.request_id,
        source: job.source,
        elapsed: started_at.elapsed(),
        result,
    }
}

fn source_add_async_enabled() -> bool {
    #[cfg(test)]
    {
        source_add_async_override_for_tests().unwrap_or(false)
    }
    #[cfg(not(test))]
    {
        true
    }
}

#[cfg(test)]
thread_local! {
    static SOURCE_ADD_ASYNC_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

#[cfg(test)]
fn source_add_async_override_for_tests() -> Option<bool> {
    SOURCE_ADD_ASYNC_OVERRIDE.with(|value| value.get())
}

#[cfg(test)]
pub(crate) fn with_source_add_async_enabled_for_tests<T>(
    enabled: bool,
    run: impl FnOnce() -> T,
) -> T {
    struct Reset<'a> {
        cell: &'a Cell<Option<bool>>,
        previous: Option<bool>,
    }

    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.cell.set(self.previous);
        }
    }

    SOURCE_ADD_ASYNC_OVERRIDE.with(|value| {
        let previous = value.replace(Some(enabled));
        let _reset = Reset {
            cell: value,
            previous,
        };
        run()
    })
}
