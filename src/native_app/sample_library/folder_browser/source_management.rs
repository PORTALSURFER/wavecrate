use std::path::PathBuf;

#[cfg(test)]
use super::scan_types::FolderScanDiscovery;
use super::{
    FolderBrowserState, FolderEntry, RemovedSource, SourceEntry,
    path_helpers::{folder_label, path_id},
    scan_types::{FolderScanDiscoveryBatch, FolderScanRequest, FolderScanResult},
    scanning::{default_root_path, load_root_folder, merge_scan_discovery, placeholder_folder},
    source_scan_cache::{load_source_scan_cache, save_source_scan_cache},
};
use wavecrate::sample_sources::{SampleSource, SourceId};

#[derive(Clone, Debug)]
pub(super) struct BrowserSourceState {
    pub(super) selected_source: String,
    pub(super) sources: Vec<SourceEntry>,
}

impl BrowserSourceState {
    pub(super) fn new(sources: Vec<SourceEntry>, selected_source: String) -> Self {
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

    #[cfg(test)]
    pub(in crate::native_app) fn from_sample_sources(sources: &[SampleSource]) -> Self {
        if sources.is_empty() {
            return Self::load_default();
        }
        let entries = sources
            .iter()
            .map(|source| {
                SourceEntry::new(
                    source.id.as_str().to_string(),
                    folder_label(&source.root),
                    source.root.clone(),
                )
            })
            .collect::<Vec<_>>();
        Self::from_sources(entries, sources[0].id.as_str().to_string())
    }

    pub(in crate::native_app) fn from_sample_sources_deferred(sources: &[SampleSource]) -> Self {
        if sources.is_empty() {
            return Self::load_default();
        }
        let scan_cache = load_source_scan_cache().unwrap_or_else(|error| {
            tracing::warn!("{error}; falling back to source disk scan");
            Default::default()
        });
        let entries = sources
            .iter()
            .map(|source| {
                let mut entry = SourceEntry::new(
                    source.id.as_str().to_string(),
                    folder_label(&source.root),
                    source.root.clone(),
                );
                entry.root_folder = scan_cache.folder_for_source(source.id.as_str(), &source.root);
                entry
            })
            .collect::<Vec<_>>();
        Self::from_sources_deferred(entries, sources[0].id.as_str().to_string())
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
    pub(super) fn source_labels_for_tests(&self) -> Vec<String> {
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

    pub(in crate::native_app) fn source_is_removable(&self, source_id: &str) -> bool {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .is_some_and(|source| !source.is_default_assets_source())
    }

    pub(in crate::native_app) fn remove_source(
        &mut self,
        source_id: &str,
    ) -> Result<RemovedSource, String> {
        let index = self
            .source
            .sources
            .iter()
            .position(|source| source.id == source_id)
            .ok_or_else(|| String::from("Source is unavailable"))?;
        if self.source.sources[index].is_default_assets_source() {
            return Err(String::from("Default source cannot be removed"));
        }
        let source = self.source.sources.remove(index);
        let removed = RemovedSource {
            label: source.label.clone(),
            root: source.root.clone(),
        };
        self.cancel_rename();
        self.clear_drag();
        if self.source.sources.is_empty() {
            self.install_default_assets_source();
        }
        if self.source.selected_source == source.id {
            self.select_first_available_source();
        }
        Ok(removed)
    }

    pub(in crate::native_app) fn begin_add_source_path(
        &mut self,
        root: PathBuf,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        if let Some(index) = self
            .source
            .sources
            .iter()
            .position(|source| source.root == root)
        {
            let id = self.source.sources[index].id.clone();
            return self.begin_select_source(id, task_id);
        }
        let id = path_id(&root);
        let label = folder_label(&root);
        let mut source = SourceEntry::new(id.clone(), label.clone(), root.clone());
        source.loading_task = Some(task_id);
        self.source.sources.push(source);
        self.select_pending_source(id.clone(), placeholder_folder(&root));
        Some(FolderScanRequest {
            task_id,
            source_id: id,
            label,
            root,
        })
    }

    pub(in crate::native_app) fn begin_select_source(
        &mut self,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        let index = self
            .source
            .sources
            .iter()
            .position(|source| source.id == id)?;
        if self.source.selected_source == id && self.source.sources[index].root_folder.is_some() {
            return None;
        }
        if let Some(root_folder) = self.source.sources[index].root_folder.clone() {
            self.select_loaded_source(id, root_folder);
            return None;
        }
        if self.source.sources[index].loading_task.is_some() {
            let root = self.source.sources[index].root.clone();
            self.select_pending_source(id, placeholder_folder(&root));
            return None;
        }
        self.source.sources[index].loading_task = Some(task_id);
        let source = self.source.sources[index].clone();
        self.select_pending_source(source.id.clone(), placeholder_folder(&source.root));
        Some(FolderScanRequest {
            task_id,
            source_id: source.id,
            label: source.label,
            root: source.root,
        })
    }

    pub(in crate::native_app) fn begin_selected_source_scan(
        &mut self,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        self.begin_source_scan(self.source.selected_source.clone(), task_id)
    }

    pub(in crate::native_app) fn begin_source_scan(
        &mut self,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        let index = self
            .source
            .sources
            .iter()
            .position(|source| source.id == id)?;
        if self.source.sources[index].loading_task.is_some() {
            return None;
        }
        self.source.sources[index].loading_task = Some(task_id);
        let source = self.source.sources[index].clone();
        Some(FolderScanRequest {
            task_id,
            source_id: source.id,
            label: source.label,
            root: source.root,
        })
    }

    pub(in crate::native_app) fn selected_source_loaded(&self) -> bool {
        self.source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)
            .is_some_and(|source| source.root_folder.is_some())
    }

    pub(in crate::native_app) fn apply_scan_finished(&mut self, result: FolderScanResult) -> bool {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == result.source_id)
        else {
            return false;
        };
        if source.loading_task != Some(result.task_id) {
            return false;
        }
        let source_id = source.id.clone();
        let should_select = self.source.selected_source == source_id;
        source.loading_task = None;
        source.root_folder = Some(result.folder.clone());
        if should_select {
            self.select_loaded_source(source_id, result.folder);
        } else {
            self.bump_file_content_revision();
        }
        true
    }

    #[cfg(test)]
    pub(super) fn apply_scan_discovered(&mut self, event: FolderScanDiscovery) -> bool {
        self.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
            task_id: event.task_id,
            source_id: event.source_id.clone(),
            events: vec![event],
        })
    }

    pub(in crate::native_app) fn apply_scan_discovered_batch(
        &mut self,
        batch: FolderScanDiscoveryBatch,
    ) -> bool {
        let Some(source) = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == batch.source_id)
        else {
            return false;
        };
        if source.loading_task != Some(batch.task_id) {
            return false;
        }

        let root_folder = source
            .root_folder
            .get_or_insert_with(|| placeholder_folder(&source.root));
        let mut changed = false;
        for event in &batch.events {
            changed |= merge_scan_discovery(root_folder, event);
        }
        if changed && self.source.selected_source == batch.source_id {
            self.tree.folders = vec![root_folder.clone()];
            self.bump_file_content_revision();
        }
        changed
    }

    fn select_pending_source(&mut self, id: String, folder: FolderEntry) {
        self.cancel_rename();
        self.selection.selected_collection = None;
        self.collection_panel.rename_edit = None;
        let root_id = folder.id.clone();
        self.source.selected_source = id;
        self.selection.select_folder(root_id.clone());
        self.reset_tree_view();
        self.reset_file_view();
        self.tree.expanded_folders.clear();
        self.tree.expanded_folders.insert(root_id);
        self.tree.folders = vec![folder];
        self.bump_file_content_revision();
    }

    fn select_loaded_source(&mut self, id: String, root_folder: FolderEntry) {
        self.cancel_rename();
        self.selection.selected_collection = None;
        self.collection_panel.rename_edit = None;
        let root_id = root_folder.id.clone();
        self.source.selected_source = id;
        self.selection.select_folder(root_id.clone());
        self.reset_tree_view();
        self.reset_file_view();
        self.tree.expanded_folders.clear();
        self.tree.expanded_folders.insert(root_id);
        self.tree.folders = vec![root_folder];
        self.bump_file_content_revision();
        self.prewarm_selected_source_audio_projection_cache();
    }

    fn install_default_assets_source(&mut self) {
        let root = default_root_path();
        let mut source = SourceEntry::new("assets", "Assets", root.clone());
        source.root_folder = Some(load_root_folder(root));
        self.source.sources.push(source);
    }

    fn select_first_available_source(&mut self) {
        let Some(source) = self.source.sources.first().cloned() else {
            return;
        };
        if let Some(root_folder) = source.root_folder {
            self.select_loaded_source(source.id, root_folder);
        } else {
            self.select_pending_source(source.id, placeholder_folder(&source.root));
        }
    }
}
