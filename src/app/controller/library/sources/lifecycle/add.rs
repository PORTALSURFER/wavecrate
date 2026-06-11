use super::telemetry::record_source_lifecycle_event;
use super::validation::{nested_source_conflict_error, source_roots_match};
use super::*;

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
        prepare_database_for_add(&source, &normalized, started_at)?;
        self.commit_added_source(source, started_at)
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

fn prepare_database_for_add(
    source: &SampleSource,
    normalized: &PathBuf,
    started_at: Instant,
) -> Result<(), String> {
    if let Err(err) = SourceDatabase::open(normalized) {
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
    Ok(())
}
