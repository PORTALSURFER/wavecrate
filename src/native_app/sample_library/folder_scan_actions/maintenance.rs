use std::path::{Path, PathBuf};

use wavecrate::sample_sources::{
    HarvestFileIdentity, HarvestFileKey, SampleSource,
    config::{AppConfig, save_if_revision_current},
    harvest_file_ops, library,
};

use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanCacheUpdate, apply_folder_scan_cache_update,
};

#[derive(Clone)]
pub(super) struct FolderScanMaintenanceRequest {
    pub(super) config: AppConfig,
    pub(super) config_revision: u64,
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
    let config_error = save_if_revision_current(&request.config, request.config_revision)
        .err()
        .map(|error| error.to_string());
    let scan_cache_error =
        apply_folder_scan_cache_update(request.scan_cache_update, request.scan_cache_revision)
            .err();
    let harvest_errors = request
        .audio_file_paths
        .iter()
        .filter_map(|path| persist_harvest_discovery(&request.sources, path).err())
        .collect();
    FolderScanMaintenanceResult {
        config_error,
        scan_cache_error,
        harvest_errors,
    }
}

fn persist_harvest_discovery(sources: &[SampleSource], path: &Path) -> Result<(), String> {
    let Some((source, relative_path)) = sources
        .iter()
        .filter_map(|source| {
            path.strip_prefix(&source.root)
                .ok()
                .map(|relative| (source, relative.to_path_buf()))
        })
        .max_by_key(|(source, _)| source.root.components().count())
    else {
        return Ok(());
    };
    let (file_size, modified_ns) = harvest_file_ops::file_identity_metadata(path);
    let entry = source
        .open_db()
        .ok()
        .and_then(|db| db.entry_for_path(&relative_path).ok().flatten());
    let identity = HarvestFileIdentity {
        key: HarvestFileKey::new(source.id.clone(), relative_path),
        file_size: file_size.or_else(|| entry.as_ref().map(|entry| entry.file_size)),
        modified_ns: modified_ns.or_else(|| entry.as_ref().map(|entry| entry.modified_ns)),
        content_hash: entry.and_then(|entry| entry.content_hash),
    };
    library::upsert_harvest_file(&identity)
        .map(|_| ())
        .map_err(|error| format!("record harvest discovery {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_outside_configured_sources_is_a_noop() {
        let result = persist_harvest_discovery(&[], Path::new("/tmp/outside.wav"));
        assert_eq!(result, Ok(()));
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
}
