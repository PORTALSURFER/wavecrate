//! Folder-browser disk-scan orchestration and refresh policy.

use super::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const AUTO_SYNC_INTERVAL: Duration = Duration::from_secs(10);

impl AppController {
    /// Apply a completed disk scan result to the folder browser cache.
    pub(crate) fn apply_folder_scan_result(
        &mut self,
        result: crate::app::controller::jobs::FolderScanResult,
    ) {
        let Some(model) = self.ui_cache.folders.models.get_mut(&result.source_id) else {
            return;
        };
        model.disk_folders = result.folders;
        model.last_disk_refresh = Some(Instant::now());
        model.disk_refresh_in_progress = false;
        if self.selection_state.ctx.selected_source.as_ref() == Some(&result.source_id) {
            self.refresh_folder_browser();
        }
    }

    #[cfg(test)]
    /// Refresh the folder browser while scanning disk folders synchronously (tests only).
    pub(crate) fn refresh_folder_browser_for_tests(&mut self) {
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            self.ui.sources.folders = FolderBrowserUiState::default();
            return;
        };
        let Some(source) = self.current_source() else {
            self.ui.sources.folders = FolderBrowserUiState::default();
            return;
        };
        let cancel = AtomicBool::new(false);
        let disk_folders = scan_disk_folders(&source.root, &cancel);
        {
            let model = self
                .ui_cache
                .folders
                .models
                .entry(source_id.clone())
                .or_default();
            model.disk_folders = disk_folders;
            model.last_disk_refresh = Some(Instant::now());
            model.disk_refresh_in_progress = false;
        }
        self.refresh_folder_browser();
    }

    pub(in crate::app) fn refresh_folder_browser_if_stale(&mut self, max_age: Duration) {
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            return;
        };
        let now = Instant::now();
        let needs_refresh = {
            let model = self
                .ui_cache
                .folders
                .models
                .entry(source_id.clone())
                .or_default();
            model
                .last_disk_refresh
                .is_none_or(|last| now.duration_since(last) >= max_age)
        };
        let pending_source = self.runtime.jobs.pending_folder_scan_source();
        if let Some(pending_source) = pending_source.as_ref()
            && pending_source != &source_id
            && let Some(model) = self.ui_cache.folders.models.get_mut(pending_source)
        {
            model.disk_refresh_in_progress = false;
        }
        if needs_refresh {
            let should_request = {
                let model = self
                    .ui_cache
                    .folders
                    .models
                    .entry(source_id.clone())
                    .or_default();
                !model.disk_refresh_in_progress
            };
            if should_request {
                if let Some(source) = self.current_source() {
                    let model = self
                        .ui_cache
                        .folders
                        .models
                        .entry(source_id.clone())
                        .or_default();
                    model.disk_refresh_in_progress = true;
                    self.runtime
                        .jobs
                        .request_folder_scan(source_id.clone(), source.root.clone());
                }
                self.refresh_folder_browser();
            }
        }
        self.request_auto_quick_sync_if_due(AUTO_SYNC_INTERVAL);
    }
}

/// Scan disk folders under `root`, honoring a cancellation signal.
pub(crate) fn scan_disk_folders(root: &Path, cancel: &AtomicBool) -> BTreeSet<PathBuf> {
    let mut folders = BTreeSet::new();
    collect_disk_folders(root, PathBuf::new(), &mut folders, cancel);
    folders
}

fn collect_disk_folders(
    root: &Path,
    parent: PathBuf,
    folders: &mut BTreeSet<PathBuf>,
    cancel: &AtomicBool,
) {
    if cancel.load(Ordering::Relaxed) {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let relative = if parent.as_os_str().is_empty() {
            PathBuf::from(name)
        } else {
            parent.join(&name)
        };
        folders.insert(relative.clone());
        collect_disk_folders(&entry.path(), relative, folders, cancel);
    }
}
