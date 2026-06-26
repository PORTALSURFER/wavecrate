use std::{collections::HashSet, path::PathBuf};

#[cfg(test)]
use super::super::scan_types::FolderScanDiscovery;
use super::super::{
    FolderBrowserState, FolderEntry, SourceEntry,
    path_helpers::{folder_label, path_id},
    scan_types::{
        FolderScanDiscoveryBatch, FolderScanRequest, FolderScanResult, FolderTreeRefreshResult,
    },
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
        let database_root = source.database_root.clone();
        self.source.sources.push(source);
        self.select_pending_source(id.clone(), placeholder_folder(&root));
        Some(FolderScanRequest {
            task_id,
            source_id: id,
            label,
            root,
            database_root,
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
        if self.source.sources[index]
            .refresh_availability_from_disk()
            .is_missing()
        {
            self.select_cached_or_placeholder_source(index);
            return None;
        }
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
            database_root: source.database_root,
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
        if self.source.sources[index]
            .refresh_availability_from_disk()
            .is_missing()
        {
            self.refresh_selected_source_from_cache_or_placeholder(index);
            return None;
        }
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
            database_root: source.database_root,
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
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == result.source_id)
        else {
            return false;
        };
        if self.source.sources[source_index].loading_task != Some(result.task_id) {
            return false;
        }
        let source_id = self.source.sources[source_index].id.clone();
        let should_select = self.source.selected_source == source_id;
        let refreshing_selected_loaded_source =
            should_select && self.source.sources[source_index].root_folder.is_some();
        self.source.sources[source_index].loading_task = None;
        if !result.source_root_available {
            self.source.sources[source_index].mark_missing();
            self.refresh_selected_source_from_cache_or_placeholder(source_index);
            return true;
        }
        self.source.sources[source_index].mark_available();
        self.source.sources[source_index].missing_collection_snapshot =
            result.missing_collection_snapshot.clone();
        self.source.sources[source_index].root_folder = Some(result.folder.clone());
        if should_select {
            if refreshing_selected_loaded_source {
                self.refresh_selected_source_tree(source_id, result.folder);
            } else {
                self.select_loaded_source(source_id, result.folder);
                self.refresh_missing_collection_state();
            }
        } else {
            self.bump_file_content_revision();
            self.refresh_missing_collection_state();
        }
        true
    }

    pub(in crate::native_app) fn apply_folder_tree_refresh_result(
        &mut self,
        result: FolderTreeRefreshResult,
    ) -> bool {
        if self.source.selected_source != result.source_id {
            return false;
        }
        let Some(source_index) = self
            .source
            .sources
            .iter()
            .position(|source| source.id == result.source_id)
        else {
            return false;
        };
        if !result.source_root_available {
            self.source.sources[source_index].mark_missing();
            self.refresh_selected_source_from_cache_or_placeholder(source_index);
            return true;
        }
        self.source.sources[source_index].mark_available();
        let Some(root_folder) = self.source.sources[source_index].root_folder.as_mut() else {
            return false;
        };
        if root_folder.id != result.folder.id {
            return false;
        }
        if !root_folder.replace_folder_structure(result.folder) {
            return false;
        }
        self.tree.folders = vec![root_folder.clone()];
        self.retain_tree_state_after_selected_source_refresh();
        self.bump_file_content_revision();
        self.refresh_missing_collection_state();
        self.prewarm_selected_source_audio_projection_cache();
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

impl FolderBrowserState {
    fn select_cached_or_placeholder_source(&mut self, source_index: usize) {
        let source = self.source.sources[source_index].clone();
        if let Some(root_folder) = source.root_folder {
            self.select_loaded_source(source.id, root_folder);
        } else {
            self.select_pending_source(source.id, placeholder_folder(&source.root));
        }
    }

    fn refresh_selected_source_from_cache_or_placeholder(&mut self, source_index: usize) {
        let source = &self.source.sources[source_index];
        if self.source.selected_source != source.id {
            return;
        }
        let root_folder = source
            .root_folder
            .clone()
            .unwrap_or_else(|| placeholder_folder(&source.root));
        self.refresh_selected_source_tree(source.id.clone(), root_folder);
    }

    fn refresh_selected_source_tree(&mut self, source_id: String, root_folder: FolderEntry) {
        self.source.selected_source = source_id;
        self.tree.folders = vec![root_folder];
        self.retain_tree_state_after_selected_source_refresh();
        self.reset_tree_view();
        self.bump_file_content_revision();
        self.refresh_missing_collection_state();
        self.prewarm_selected_source_audio_projection_cache();
    }

    fn retain_tree_state_after_selected_source_refresh(&mut self) {
        let root_id = self
            .tree
            .folders
            .first()
            .map(|folder| folder.id.clone())
            .unwrap_or_default();
        if root_id.is_empty() {
            return;
        }

        let still_available = self
            .tree
            .expanded_folders
            .iter()
            .filter(|id| self.find_folder(id).is_some())
            .cloned()
            .collect();
        self.tree.expanded_folders = still_available;
        self.tree.expanded_folders.insert(root_id.clone());

        if self.find_folder(&self.selection.selected_folder).is_some() {
            let mut existing_folder_ids = HashSet::new();
            for folder in &self.tree.folders {
                folder.collect_folder_ids(&mut existing_folder_ids);
            }
            self.selection
                .retain_existing_folders(&existing_folder_ids, root_id.clone());
            self.expand_selected_folder_ancestors();
            let visible_ids = self
                .selected_audio_files()
                .into_iter()
                .map(|file| file.id.clone())
                .collect();
            self.selection.retain_visible_files(&visible_ids);
            return;
        }

        self.selection.select_folder_after_tree_changed(root_id);
    }

    fn expand_selected_folder_ancestors(&mut self) {
        let selected = std::path::PathBuf::from(&self.selection.selected_folder);
        let mut cursor = selected.parent();
        while let Some(parent) = cursor {
            let id = path_id(parent);
            if self.find_folder(&id).is_some() {
                self.tree.expanded_folders.insert(id);
            }
            cursor = parent.parent();
        }
    }
}
