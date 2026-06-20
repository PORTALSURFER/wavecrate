use std::path::PathBuf;

use super::super::{
    FolderBrowserState, SourceEntry, scan::FolderTreeRefreshRequest,
    source_scan_cache::save_source_scan_cache,
};
use wavecrate::sample_sources::{SampleSource, SourceId};

#[derive(Clone, Debug)]
pub(in crate::native_app::sample_library::folder_browser) struct BrowserSourceState {
    pub(in crate::native_app::sample_library::folder_browser) selected_source: String,
    pub(in crate::native_app::sample_library::folder_browser) sources: Vec<SourceEntry>,
}

impl BrowserSourceState {
    pub(in crate::native_app::sample_library::folder_browser) fn new(
        sources: Vec<SourceEntry>,
        selected_source: String,
    ) -> Self {
        Self {
            selected_source,
            sources,
        }
    }
}

impl FolderBrowserState {
    pub(in crate::native_app) fn sources(&self) -> &[SourceEntry] {
        self.source.sources.as_slice()
    }

    pub(in crate::native_app) fn selected_source_id(&self) -> &str {
        self.source.selected_source.as_str()
    }

    pub(in crate::native_app) fn configured_sample_sources(&self) -> Vec<SampleSource> {
        self.source
            .sources
            .iter()
            .filter(|source| !source.is_default_assets_source())
            .map(|source| {
                SampleSource::new_with_id(
                    SourceId::from_string(source.id.clone()),
                    source.root.clone(),
                )
            })
            .collect()
    }

    pub(in crate::native_app) fn save_source_scan_cache(&self) -> Result<(), String> {
        save_source_scan_cache(&self.source.sources)
    }

    #[cfg(test)]
    pub(in crate::native_app::sample_library::folder_browser) fn source_labels_for_tests(
        &self,
    ) -> Vec<String> {
        self.source
            .sources
            .iter()
            .map(|source| source.label.clone())
            .collect()
    }

    pub(in crate::native_app) fn source_root_path(&self, source_id: &str) -> Option<PathBuf> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .map(|source| source.root.clone())
    }

    pub(in crate::native_app) fn selected_source_folder_tree_refresh_request(
        &self,
    ) -> Option<FolderTreeRefreshRequest> {
        let source = self
            .source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)?;
        source.root_folder.as_ref()?;
        Some(FolderTreeRefreshRequest {
            source_id: source.id.clone(),
            label: source.label.clone(),
            root: source.root.clone(),
        })
    }

    pub(in crate::native_app) fn source_relative_file_path(
        &self,
        file_path: &std::path::Path,
    ) -> Option<(PathBuf, PathBuf)> {
        self.source
            .sources
            .iter()
            .filter_map(|source| {
                file_path
                    .strip_prefix(&source.root)
                    .ok()
                    .map(|relative| (source.root.clone(), relative.to_path_buf()))
            })
            .max_by_key(|(root, _)| root.components().count())
    }

    pub(in crate::native_app) fn sample_source_for_file_path(
        &self,
        file_path: &std::path::Path,
    ) -> Option<(SampleSource, PathBuf)> {
        self.source
            .sources
            .iter()
            .filter_map(|source| {
                file_path.strip_prefix(&source.root).ok().map(|relative| {
                    (
                        SampleSource::new_with_id(
                            SourceId::from_string(source.id.clone()),
                            source.root.clone(),
                        ),
                        relative.to_path_buf(),
                    )
                })
            })
            .max_by_key(|(source, _)| source.root.components().count())
    }

    pub(in crate::native_app) fn source_is_removable(&self, source_id: &str) -> bool {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .is_some_and(|source| !source.is_default_assets_source())
    }
}
