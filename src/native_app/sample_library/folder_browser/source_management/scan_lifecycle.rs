use std::path::PathBuf;

#[cfg(test)]
use super::super::scan_types::FolderScanDiscovery;
use super::super::{
    FolderBrowserState, SourceEntry,
    path_helpers::{folder_label, path_id},
    scan_types::{FolderScanDiscoveryBatch, FolderScanRequest, FolderScanResult},
    scanning::{merge_scan_discovery, placeholder_folder},
};

impl FolderBrowserState {
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
    pub(in crate::native_app::sample_library::folder_browser) fn apply_scan_discovered(
        &mut self,
        event: FolderScanDiscovery,
    ) -> bool {
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
}
