use std::path::{Path, PathBuf};

use super::{
    super::{
        FolderBrowserState, SourceEntry, scan::FolderTreeRefreshRequest,
        source_scan_cache::save_source_scan_cache,
    },
    reorder::SourceReorderDrag,
};
use wavecrate::sample_sources::{SampleSource, SourceRole};

#[derive(Clone, Debug)]
pub(in crate::native_app::sample_library::folder_browser) struct BrowserSourceState {
    pub(in crate::native_app::sample_library::folder_browser) selected_source: String,
    pub(in crate::native_app::sample_library::folder_browser) selected_tree_loaded: bool,
    pub(in crate::native_app::sample_library::folder_browser) sources: Vec<SourceEntry>,
    pub(in crate::native_app::sample_library::folder_browser) reorder_drag:
        Option<SourceReorderDrag>,
}

impl BrowserSourceState {
    pub(in crate::native_app::sample_library::folder_browser) fn new(
        sources: Vec<SourceEntry>,
        selected_source: String,
        selected_tree_loaded: bool,
    ) -> Self {
        Self {
            selected_source,
            selected_tree_loaded,
            sources,
            reorder_drag: None,
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

    pub(in crate::native_app) fn source_label(&self, source_id: &str) -> Option<&str> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .map(|source| source.label.as_str())
    }

    pub(in crate::native_app) fn configured_sample_sources(&self) -> Vec<SampleSource> {
        self.source
            .sources
            .iter()
            .filter(|source| !source.is_default_assets_source())
            .map(SourceEntry::as_sample_source)
            .collect()
    }

    pub(in crate::native_app) fn save_source_scan_cache(&self) -> Result<(), String> {
        let mut sources = self.source.sources.clone();
        if let Some(active_root) = self.tree.folders.first()
            && let Some(source) = sources
                .iter_mut()
                .find(|source| source.id == self.source.selected_source)
            && source.root_folder.is_none()
        {
            source.root_folder = Some(active_root.clone());
        }
        save_source_scan_cache(&sources)
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

    pub(in crate::native_app) fn source_id_for_root_path(&self, root: &Path) -> Option<String> {
        self.source
            .sources
            .iter()
            .find(|source| source.root == root)
            .map(|source| source.id.clone())
    }

    pub(in crate::native_app) fn refresh_source_availability_from_disk(
        &mut self,
        source_id: &str,
    ) -> Option<bool> {
        let source = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == source_id)?;
        Some(source.refresh_availability_from_disk().is_missing())
    }

    pub(in crate::native_app) fn apply_observed_source_availability(
        &mut self,
        source_id: &str,
        available: bool,
    ) -> Option<bool> {
        let source = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == source_id)?;
        Some(source.apply_observed_availability(available).is_missing())
    }

    pub(in crate::native_app) fn source_roots(
        &self,
        source_id: &str,
    ) -> Option<(PathBuf, PathBuf)> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .map(|source| (source.root.clone(), source.database_root.clone()))
    }

    pub(in crate::native_app) fn source_roots_for_path(
        &self,
        path: &std::path::Path,
    ) -> Option<(PathBuf, PathBuf)> {
        self.source
            .sources
            .iter()
            .filter(|source| path.starts_with(&source.root))
            .max_by_key(|source| source.root.components().count())
            .map(|source| (source.root.clone(), source.database_root.clone()))
    }

    pub(in crate::native_app) fn selected_source_folder_tree_refresh_request(
        &self,
    ) -> Option<FolderTreeRefreshRequest> {
        let source = self
            .source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)?;
        if source.is_missing() {
            return None;
        }
        self.selected_source_root_folder()?;
        Some(FolderTreeRefreshRequest {
            source_id: source.id.clone(),
            label: source.label.clone(),
            root: source.root.clone(),
            database_root: source.database_root.clone(),
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

    pub(in crate::native_app) fn source_database_relative_file_path(
        &self,
        file_path: &std::path::Path,
    ) -> Option<(PathBuf, PathBuf, PathBuf)> {
        self.source
            .sources
            .iter()
            .filter_map(|source| {
                file_path.strip_prefix(&source.root).ok().map(|relative| {
                    (
                        source.root.clone(),
                        source.database_root.clone(),
                        relative.to_path_buf(),
                    )
                })
            })
            .max_by_key(|(root, _, _)| root.components().count())
    }

    pub(in crate::native_app) fn sample_source_for_file_path(
        &self,
        file_path: &std::path::Path,
    ) -> Option<(SampleSource, PathBuf)> {
        self.source
            .sources
            .iter()
            .filter_map(|source| {
                file_path
                    .strip_prefix(&source.root)
                    .ok()
                    .map(|relative| (source.as_sample_source(), relative.to_path_buf()))
            })
            .max_by_key(|(source, _)| source.root.components().count())
    }

    pub(in crate::native_app) fn primary_sample_source(&self) -> Option<SampleSource> {
        self.source
            .sources
            .iter()
            .find(|source| source.is_primary())
            .map(SourceEntry::as_sample_source)
    }

    pub(in crate::native_app) fn default_writable_extraction_source(
        &self,
        error: impl Into<String>,
    ) -> Result<SampleSource, String> {
        let error = error.into();
        if let Some(source) = self.primary_sample_source() {
            return Ok(source);
        }

        let mut normal_sources = self.source.sources.iter().filter(|source| {
            !source.is_default_assets_source()
                && !source.is_missing()
                && source.role == SourceRole::Normal
        });
        let Some(source) = normal_sources.next() else {
            return Err(error);
        };
        if normal_sources.next().is_some() {
            return Err(error);
        }
        Ok(source.as_sample_source())
    }

    pub(in crate::native_app) fn source_is_removable(&self, source_id: &str) -> bool {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .is_some_and(|source| !source.is_default_assets_source())
    }

    pub(in crate::native_app) fn source_role(&self, source_id: &str) -> Option<SourceRole> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .map(|source| source.role)
    }

    pub(in crate::native_app) fn source_is_missing(&self, source_id: &str) -> bool {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .is_some_and(SourceEntry::is_missing)
    }

    pub(in crate::native_app) fn selected_source_status_label(&self) -> Option<String> {
        let source = self
            .source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)?;
        let mut labels = Vec::new();
        if source.is_missing() {
            labels.push("Source missing");
        }
        if let Some(role) = source_role_status_label(source.role) {
            labels.push(role);
        }
        if labels.is_empty() {
            None
        } else {
            Some(labels.join(" | "))
        }
    }

    pub(in crate::native_app) fn set_source_protected(
        &mut self,
        source_id: &str,
        protected: bool,
    ) -> Result<&'static str, String> {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == source_id)
        else {
            return Err(String::from("Source is unavailable"));
        };
        let role = if protected {
            SourceRole::Protected
        } else {
            SourceRole::Normal
        };
        source.apply_role(role)?;
        Ok(if protected {
            "Protected source"
        } else {
            "Unprotected source"
        })
    }

    pub(in crate::native_app) fn set_primary_source(
        &mut self,
        source_id: &str,
    ) -> Result<&'static str, String> {
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == source_id)
        else {
            return Err(String::from("Source is unavailable"));
        };
        if self.source.sources[source_index].is_protected() {
            return Err(String::from("Primary sources must be writable."));
        }
        for source in &mut self.source.sources {
            if source.role == SourceRole::Primary {
                source.apply_role(SourceRole::Normal)?;
            }
        }
        self.source.sources[source_index].apply_role(SourceRole::Primary)?;
        Ok("Primary library")
    }

    pub(in crate::native_app) fn clear_primary_source(
        &mut self,
        source_id: &str,
    ) -> Result<&'static str, String> {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == source_id)
        else {
            return Err(String::from("Source is unavailable"));
        };
        if source.is_primary() {
            source.apply_role(SourceRole::Normal)?;
        }
        Ok("Cleared primary library")
    }
}

fn source_role_status_label(role: SourceRole) -> Option<&'static str> {
    match role {
        SourceRole::Protected => Some("Protected source"),
        SourceRole::Primary => Some("Primary library"),
        SourceRole::Normal => None,
    }
}
