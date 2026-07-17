use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use wavecrate::sample_sources::{SourceDatabase, readiness::ReadinessStage};

use super::watcher_echo::capture_watcher_echoes;
use super::{
    CommittedFileMutation, FileMutationChange, FileMutationFailure, FileMutationOperation,
    FileMutationOutcome, FileMutationSemantics,
};
use crate::native_app::sample_library::folder_scan_actions::sync_source_database_paths;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceMutationRequest {
    pub(super) source_id: String,
    pub(super) root: PathBuf,
    pub(super) database_root: PathBuf,
    pub(super) operation_id: u64,
    pub(super) operation: FileMutationOperation,
    pub(super) changes: Vec<FileMutationChange>,
    pub(super) affected_relative_paths: Vec<PathBuf>,
}

pub(super) fn mutation_completion_is_stale_or_duplicate(
    accepted: (u64, u64),
    candidate: (u64, u64),
) -> bool {
    accepted != (0, 0) && candidate <= accepted
}

pub(super) fn merge_file_mutation_failures(
    outcome: FileMutationOutcome,
    mut reported_failures: Vec<FileMutationFailure>,
) -> FileMutationOutcome {
    if reported_failures.is_empty() {
        return outcome;
    }
    match outcome {
        FileMutationOutcome::Committed(committed) => FileMutationOutcome::Failed {
            committed,
            failures: reported_failures,
        },
        FileMutationOutcome::Failed {
            committed,
            mut failures,
        } => {
            failures.append(&mut reported_failures);
            FileMutationOutcome::Failed {
                committed,
                failures,
            }
        }
        FileMutationOutcome::RolledBack(failure) => {
            reported_failures.insert(0, failure);
            FileMutationOutcome::Failed {
                committed: Vec::new(),
                failures: reported_failures,
            }
        }
    }
}

pub(super) fn capture_expected_after_identities(changes: &mut [FileMutationChange]) {
    for change in changes {
        if change.after_content_identity.is_none() {
            change.after_content_identity = change
                .after_path
                .as_deref()
                .and_then(cache_content_identity);
        }
    }
}

pub(in crate::native_app) fn cache_content_identity(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified_ns = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(format!("cache:{}:{modified_ns}", metadata.len()))
}

pub(super) fn build_source_requests(
    operation_id: u64,
    operation: FileMutationOperation,
    changes: Vec<FileMutationChange>,
    sources: &[wavecrate::sample_sources::SampleSource],
) -> Vec<SourceMutationRequest> {
    let mut grouped = BTreeMap::<String, SourceMutationRequest>::new();
    for change in changes {
        let affected_sources = [change.before_path.as_deref(), change.after_path.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(|path| source_for_path(sources, path))
            .collect::<BTreeSet<_>>();
        for source_id in affected_sources {
            let Some(source) = sources
                .iter()
                .find(|source| source.id.as_str() == source_id)
            else {
                continue;
            };
            let Ok(database_root) = source.database_root() else {
                continue;
            };
            let request =
                grouped
                    .entry(source_id.clone())
                    .or_insert_with(|| SourceMutationRequest {
                        source_id: source_id.clone(),
                        root: source.root.clone(),
                        database_root,
                        operation_id,
                        operation,
                        changes: Vec::new(),
                        affected_relative_paths: Vec::new(),
                    });
            for path in [change.before_path.as_deref(), change.after_path.as_deref()]
                .into_iter()
                .flatten()
            {
                if let Ok(relative) = path.strip_prefix(&source.root)
                    && !request
                        .affected_relative_paths
                        .iter()
                        .any(|existing| existing == relative)
                {
                    request.affected_relative_paths.push(relative.to_path_buf());
                }
            }
            request.changes.push(change.clone());
        }
    }
    grouped.into_values().collect()
}

fn source_for_path(
    sources: &[wavecrate::sample_sources::SampleSource],
    path: &Path,
) -> Option<String> {
    sources
        .iter()
        .filter_map(|source| {
            path.strip_prefix(&source.root)
                .ok()
                .map(|relative| (source, relative))
        })
        .max_by_key(|(source, _)| source.root.components().count())
        .map(|(source, _)| source.id.as_str().to_string())
}

pub(super) fn reconcile_file_mutation_requests(
    requests: Vec<SourceMutationRequest>,
) -> FileMutationOutcome {
    let cancel = AtomicBool::new(false);
    let mut committed = Vec::new();
    let mut failures = Vec::new();
    for request in requests {
        match reconcile_source_mutation(request.clone(), &cancel) {
            Ok(event) => committed.push(event),
            Err(error) => failures.push(FileMutationFailure {
                source_id: Some(request.source_id),
                operation_id: request.operation_id,
                operation: request.operation,
                error,
            }),
        }
    }
    if failures.is_empty() {
        FileMutationOutcome::Committed(committed)
    } else {
        FileMutationOutcome::Failed {
            committed,
            failures,
        }
    }
}

pub(super) fn reconcile_source_mutation(
    request: SourceMutationRequest,
    cancel: &AtomicBool,
) -> Result<CommittedFileMutation, String> {
    let watcher_echoes = capture_watcher_echoes(&request.root, &request.affected_relative_paths);
    let before_database = SourceDatabase::open_for_background_job_with_database_root(
        &request.root,
        &request.database_root,
    )
    .map_err(|error| format!("open source before mutation reconciliation: {error}"))?;
    let before = manifest_by_path(
        before_database
            .list_manifest_entries()
            .map_err(|error| format!("read source manifest before mutation: {error}"))?,
    );
    drop(before_database);

    let sync = sync_source_database_paths(
        request.source_id.clone(),
        request.root.clone(),
        request.database_root.clone(),
        request.affected_relative_paths.clone(),
        request.affected_relative_paths.len(),
        cancel,
    );
    let success = sync.result?;
    if let Some(error) = success.incomplete_error {
        return Err(format!(
            "source mutation reconciliation incomplete: {error}"
        ));
    }

    let after_database = SourceDatabase::open_for_background_job_with_database_root(
        &request.root,
        &request.database_root,
    )
    .map_err(|error| format!("open source after mutation reconciliation: {error}"))?;
    let after = manifest_by_path(
        after_database
            .list_manifest_entries()
            .map_err(|error| format!("read source manifest after mutation: {error}"))?,
    );
    let committed_source_revision = after_database
        .get_revision()
        .map_err(|error| format!("read committed source revision: {error}"))?
        .max(success.committed_delta.revision);

    verify_mutation_still_matches_filesystem(&request.changes)?;
    let changes = request
        .changes
        .into_iter()
        .map(|mut change| {
            let before_entry = change
                .before_path
                .as_deref()
                .and_then(|path| path.strip_prefix(&request.root).ok())
                .and_then(|path| before.get(path));
            let after_entry = change
                .after_path
                .as_deref()
                .and_then(|path| path.strip_prefix(&request.root).ok())
                .and_then(|path| after.get(path));
            if change.before_content_identity.is_none() {
                change.before_content_identity = before_entry.map(content_identity);
            }
            if change
                .after_content_identity
                .as_deref()
                .is_none_or(|identity| identity.starts_with("cache:"))
            {
                change.after_content_identity = after_entry.map(content_identity);
            }
            if change.semantics == FileMutationSemantics::PathOnlyMove {
                if change.before_content_identity.is_none() {
                    change.before_content_identity = change.after_content_identity.clone();
                }
                if change.after_content_identity.is_none() {
                    change.after_content_identity = change.before_content_identity.clone();
                }
            }
            change
        })
        .collect::<Vec<_>>();

    Ok(CommittedFileMutation {
        source_id: request.source_id,
        operation_id: request.operation_id,
        operation: request.operation,
        committed_source_revision,
        invalidated_stages: invalidated_stages(&changes),
        changes,
        committed_delta: success.committed_delta,
        affected_relative_paths: request.affected_relative_paths,
        watcher_echoes,
    })
}

fn verify_mutation_still_matches_filesystem(changes: &[FileMutationChange]) -> Result<(), String> {
    for change in changes {
        if change.semantics == FileMutationSemantics::Delete {
            if change.before_path.as_deref().is_some_and(Path::exists) {
                return Err(String::from(
                    "committed delete was superseded before source reconciliation",
                ));
            }
            continue;
        }
        let expected = change
            .after_content_identity
            .as_deref()
            .filter(|identity| identity.starts_with("cache:"));
        if let Some(expected) = expected {
            let current = change
                .after_path
                .as_deref()
                .and_then(cache_content_identity);
            if current.as_deref() != Some(expected) {
                return Err(String::from(
                    "committed mutation was superseded before source reconciliation",
                ));
            }
        }
    }
    Ok(())
}

fn manifest_by_path(
    entries: Vec<wavecrate_library::sample_sources::SourceManifestEntry>,
) -> HashMap<PathBuf, wavecrate_library::sample_sources::SourceManifestEntry> {
    entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect()
}

fn content_identity(entry: &wavecrate_library::sample_sources::SourceManifestEntry) -> String {
    entry
        .content_hash
        .as_deref()
        .filter(|hash| !hash.trim().is_empty())
        .map(|hash| format!("hash:{hash}"))
        .or_else(|| {
            entry
                .file_identity
                .as_deref()
                .filter(|identity| !identity.trim().is_empty())
                .map(|identity| {
                    format!(
                        "pending:{identity}:{}:{}",
                        entry.file_size, entry.modified_ns
                    )
                })
        })
        .unwrap_or_else(|| {
            format!(
                "pending:{}:{}:{}",
                entry.relative_path.display(),
                entry.file_size,
                entry.modified_ns
            )
        })
}

fn invalidated_stages(changes: &[FileMutationChange]) -> BTreeSet<ReadinessStage> {
    let mut stages = BTreeSet::new();
    for change in changes {
        match change.semantics {
            FileMutationSemantics::Create | FileMutationSemantics::ContentChanged => {
                stages.extend([
                    ReadinessStage::IndexedIdentity,
                    ReadinessStage::PlaybackSummary,
                    ReadinessStage::AnalysisFeatures,
                    ReadinessStage::EmbeddingAspects,
                    ReadinessStage::SimilarityLayout,
                ]);
            }
            FileMutationSemantics::PathOnlyMove => {}
            FileMutationSemantics::Delete => {
                stages.insert(ReadinessStage::SimilarityLayout);
            }
        }
    }
    stages
}
