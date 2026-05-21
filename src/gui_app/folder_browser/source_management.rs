use std::path::PathBuf;

use super::{
    FolderBrowserState, FolderEntry, SourceEntry,
    path_helpers::{folder_label, path_id},
    scanning::{merge_scan_discovery, placeholder_folder},
    types::{FolderScanDiscoveryBatch, FolderScanRequest, FolderScanResult},
};

impl FolderBrowserState {
    #[cfg(test)]
    pub(super) fn source_labels_for_tests(&self) -> Vec<String> {
        self.sources
            .iter()
            .map(|source| source.label.clone())
            .collect()
    }

    pub(in crate::gui_app) fn source_root_path(&self, source_id: &str) -> Option<PathBuf> {
        self.sources
            .iter()
            .find(|source| source.id == source_id)
            .map(|source| source.root.clone())
    }

    pub(in crate::gui_app) fn begin_add_source_path(
        &mut self,
        root: PathBuf,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        if let Some(index) = self.sources.iter().position(|source| source.root == root) {
            let id = self.sources[index].id.clone();
            return self.begin_select_source(id, task_id);
        }
        let id = path_id(&root);
        let label = folder_label(&root);
        let mut source = SourceEntry::new(id.clone(), label.clone(), root.clone());
        source.loading_task = Some(task_id);
        self.sources.push(source);
        self.select_pending_source(id.clone(), placeholder_folder(&root));
        Some(FolderScanRequest {
            task_id,
            source_id: id,
            label,
            root,
        })
    }

    pub(in crate::gui_app) fn begin_select_source(
        &mut self,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        let index = self.sources.iter().position(|source| source.id == id)?;
        if self.selected_source == id && self.sources[index].root_folder.is_some() {
            return None;
        }
        if let Some(root_folder) = self.sources[index].root_folder.clone() {
            self.select_loaded_source(id, root_folder);
            return None;
        }
        if self.sources[index].loading_task.is_some() {
            let root = self.sources[index].root.clone();
            self.select_pending_source(id, placeholder_folder(&root));
            return None;
        }
        self.sources[index].loading_task = Some(task_id);
        let source = self.sources[index].clone();
        self.select_pending_source(source.id.clone(), placeholder_folder(&source.root));
        Some(FolderScanRequest {
            task_id,
            source_id: source.id,
            label: source.label,
            root: source.root,
        })
    }

    pub(in crate::gui_app) fn apply_scan_finished(&mut self, result: FolderScanResult) -> bool {
        let Some(source) = self
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
        let should_select = self.selected_source == source_id;
        source.loading_task = None;
        source.root_folder = Some(result.folder.clone());
        if should_select {
            self.select_loaded_source(source_id, result.folder);
        }
        true
    }

    #[cfg(test)]
    pub(super) fn apply_scan_discovered(
        &mut self,
        event: super::types::FolderScanDiscovery,
    ) -> bool {
        self.apply_scan_discovered_batch(FolderScanDiscoveryBatch {
            task_id: event.task_id,
            source_id: event.source_id.clone(),
            events: vec![event],
        })
    }

    pub(in crate::gui_app) fn apply_scan_discovered_batch(
        &mut self,
        batch: FolderScanDiscoveryBatch,
    ) -> bool {
        let Some(source) = self
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
        if changed && self.selected_source == batch.source_id {
            self.folders = vec![root_folder.clone()];
        }
        changed
    }

    fn select_pending_source(&mut self, id: String, folder: FolderEntry) {
        self.cancel_rename();
        let root_id = folder.id.clone();
        self.selected_source = id;
        self.selected_folder = root_id.clone();
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root_id);
        self.folders = vec![folder];
    }

    fn select_loaded_source(&mut self, id: String, root_folder: FolderEntry) {
        self.cancel_rename();
        let root_id = root_folder.id.clone();
        self.selected_source = id;
        self.selected_folder = root_id.clone();
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
        self.expanded_folders.clear();
        self.expanded_folders.insert(root_id);
        self.folders = vec![root_folder];
    }
}
