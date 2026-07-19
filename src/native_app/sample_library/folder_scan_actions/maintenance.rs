use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use wavecrate::sample_sources::{
    HarvestFileIdentity, HarvestFileKey, SampleSource,
    config::{AppConfig, ConfigError, ConfigSaveRevision, save_if_revision_current},
    harvest_file_ops, library,
};

use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanCacheUpdate, apply_folder_scan_cache_update,
};

#[derive(Clone)]
pub(super) struct FolderScanMaintenanceRequest {
    pub(super) config: AppConfig,
    pub(super) config_revision: Result<ConfigSaveRevision, String>,
    pub(super) sources: Vec<SampleSource>,
    pub(super) audio_file_paths: Vec<PathBuf>,
    pub(super) scan_cache_update: FolderScanCacheUpdate,
    pub(super) scan_cache_revision: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanMaintenanceResult {
    pub(in crate::native_app) config_error: Option<String>,
    pub(in crate::native_app) scan_cache_error: Option<String>,
    pub(in crate::native_app) harvest_errors: Vec<String>,
}

impl FolderScanMaintenanceResult {
    pub(super) fn persistence_error(&self) -> Option<String> {
        match (&self.config_error, &self.scan_cache_error) {
            (Some(config), Some(cache)) => Some(format!(
                "source configuration: {config}; source scan cache: {cache}"
            )),
            (Some(config), None) => Some(format!("source configuration: {config}")),
            (None, Some(cache)) => Some(format!("source scan cache: {cache}")),
            (None, None) => None,
        }
    }
}

pub(super) fn persist_folder_scan_maintenance(
    request: FolderScanMaintenanceRequest,
) -> FolderScanMaintenanceResult {
    // Persist the browser snapshot first. Configuration and harvest reconciliation can touch large
    // source databases; if shutdown interrupts that follow-up work, the next launch should still
    // be able to restore the completed scan instead of scanning the same source again.
    let scan_cache_error =
        apply_folder_scan_cache_update(request.scan_cache_update, request.scan_cache_revision)
            .err();
    let config_error = persist_config_revision(&request.config, &request.config_revision);
    let harvest_errors = persist_harvest_discoveries(&request.sources, &request.audio_file_paths);
    FolderScanMaintenanceResult {
        config_error,
        scan_cache_error,
        harvest_errors,
    }
}

fn persist_config_revision(
    config: &AppConfig,
    revision: &Result<ConfigSaveRevision, String>,
) -> Option<String> {
    match revision {
        Ok(revision) => match save_if_revision_current(config, revision) {
            Ok(true) => None,
            Ok(false) => Some(ConfigError::SaveSuperseded.to_string()),
            Err(error) => Some(error.to_string()),
        },
        Err(error) => Some(error.clone()),
    }
}

fn persist_harvest_discoveries(sources: &[SampleSource], paths: &[PathBuf]) -> Vec<String> {
    let mut paths_by_source = BTreeMap::<usize, Vec<(&Path, PathBuf)>>::new();
    for path in paths {
        let Some((source_index, relative_path)) = sources
            .iter()
            .enumerate()
            .filter_map(|(index, source)| {
                path.strip_prefix(&source.root)
                    .ok()
                    .map(|relative| (index, relative.to_path_buf()))
            })
            .max_by_key(|(index, _)| sources[*index].root.components().count())
        else {
            continue;
        };
        paths_by_source
            .entry(source_index)
            .or_default()
            .push((path.as_path(), relative_path));
    }

    let mut identities = Vec::with_capacity(paths.len());
    for (source_index, source_paths) in paths_by_source {
        let source = &sources[source_index];
        let manifest = source
            .open_db()
            .ok()
            .and_then(|db| db.list_manifest_entries().ok())
            .unwrap_or_default()
            .into_iter()
            .map(|entry| (entry.relative_path.clone(), entry))
            .collect::<HashMap<_, _>>();
        identities.extend(source_paths.into_iter().map(|(path, relative_path)| {
            let (file_size, modified_ns) = harvest_file_ops::file_identity_metadata(path);
            let entry = manifest.get(&relative_path);
            HarvestFileIdentity {
                key: HarvestFileKey::new(source.id.clone(), relative_path),
                file_size: file_size.or_else(|| entry.map(|entry| entry.file_size)),
                modified_ns: modified_ns.or_else(|| entry.map(|entry| entry.modified_ns)),
                content_hash: entry.and_then(|entry| entry.content_hash.clone()),
            }
        }));
    }

    library::upsert_harvest_files(&identities)
        .err()
        .map(|error| format!("record {} harvest discoveries: {error}", identities.len()))
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_outside_configured_sources_is_a_noop() {
        let errors = persist_harvest_discoveries(&[], &[PathBuf::from("/tmp/outside.wav")]);
        assert!(errors.is_empty());
    }

    #[test]
    fn discovery_batch_opens_each_source_database_once() {
        let config = tempfile::tempdir().unwrap();
        let _guard = wavecrate::app_dirs::ConfigBaseGuard::set(config.path().to_path_buf());
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first.wav");
        let second = root.path().join("second.wav");
        std::fs::write(&first, b"first").unwrap();
        std::fs::write(&second, b"second").unwrap();
        let source = SampleSource::new(root.path().to_path_buf());
        wavecrate::sample_sources::db::test_reset_source_db_open_total_count(root.path());

        let errors = persist_harvest_discoveries(&[source], &[first, second]);

        assert!(errors.is_empty());
        assert_eq!(
            wavecrate::sample_sources::db::test_source_db_open_total_count(root.path()),
            1
        );
    }

    #[test]
    fn maintenance_result_combines_user_visible_persistence_errors() {
        let result = FolderScanMaintenanceResult {
            config_error: Some(String::from("config denied")),
            scan_cache_error: Some(String::from("cache full")),
            harvest_errors: Vec::new(),
        };

        assert_eq!(
            result.persistence_error().as_deref(),
            Some("source configuration: config denied; source scan cache: cache full")
        );
    }

    #[test]
    fn superseded_config_revision_is_reported_as_persistence_error() {
        let root = tempfile::tempdir().unwrap();
        let _guard = wavecrate::app_dirs::ConfigBaseGuard::set(root.path().to_path_buf());
        let stale = wavecrate::sample_sources::config::reserve_save_revision().unwrap();
        let _current = wavecrate::sample_sources::config::reserve_save_revision().unwrap();

        let error = persist_config_revision(&AppConfig::default(), &Ok(stale));

        assert!(error.is_some_and(|error| error.contains("superseded")));
    }
}
