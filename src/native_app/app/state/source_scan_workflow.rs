use std::{collections::HashSet, path::PathBuf};

use crate::native_app::sample_library::folder_browser::{
    FolderBrowserState,
    scan::{FolderScanDiscoveryBatch, FolderScanProgress, FolderScanRequest, FolderScanResult},
};

#[cfg(test)]
#[path = "source_scan_workflow/tests.rs"]
/// Source-scan workflow state tests split out from the runtime module.
mod tests;

pub(in crate::native_app) struct SourceScanWorkflow {
    progress: Option<FolderScanProgress>,
    pending_refreshes: HashSet<String>,
}

pub(in crate::native_app) enum SourceFilesystemChangePlan {
    IgnoredSourceMissing {
        source_id: String,
    },
    Patched {
        source_id: String,
        changed_count: usize,
        changed: bool,
    },
    DeferredAlreadyRunning {
        source_id: String,
    },
    QueueRefresh {
        source_id: String,
    },
}

pub(in crate::native_app) enum SourceRefreshRequest {
    Queued(FolderScanRequest),
    Deferred { source_id: String },
}

pub(in crate::native_app) enum SourceScanFinish {
    Applied {
        source_id: String,
        label: String,
        file_count: usize,
        folder_count: usize,
        source_db_error: Option<String>,
    },
    Stale {
        label: String,
    },
}

impl SourceScanWorkflow {
    pub(in crate::native_app) fn new() -> Self {
        Self {
            progress: None,
            pending_refreshes: HashSet::new(),
        }
    }

    pub(in crate::native_app) fn progress(&self) -> Option<&FolderScanProgress> {
        self.progress.as_ref()
    }

    pub(in crate::native_app) fn active(&self) -> bool {
        self.progress.is_some()
    }

    pub(in crate::native_app) fn begin_add_source_path(
        &mut self,
        browser: &mut FolderBrowserState,
        root: PathBuf,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        browser.begin_add_source_path(root, task_id)
    }

    pub(in crate::native_app) fn begin_select_source(
        &mut self,
        browser: &mut FolderBrowserState,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        browser.begin_select_source(id, task_id)
    }

    pub(in crate::native_app) fn begin_source_scan(
        &mut self,
        browser: &mut FolderBrowserState,
        id: String,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        browser.begin_source_scan(id, task_id)
    }

    pub(in crate::native_app) fn begin_selected_source_scan(
        &mut self,
        browser: &mut FolderBrowserState,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        browser.begin_selected_source_scan(task_id)
    }

    pub(in crate::native_app) fn start_scan(&mut self, request: &FolderScanRequest) {
        self.progress = Some(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Queued"),
            completed: 0,
            total: 0,
            detail: request.root.display().to_string(),
        });
    }

    pub(in crate::native_app) fn apply_progress(
        &mut self,
        browser: &FolderBrowserState,
        progress: FolderScanProgress,
    ) -> bool {
        if !browser.scan_is_active(&progress.source_id, progress.task_id) {
            return false;
        }
        self.progress = Some(progress);
        true
    }

    pub(in crate::native_app) fn apply_discovery_batch(
        &mut self,
        browser: &mut FolderBrowserState,
        batch: FolderScanDiscoveryBatch,
    ) -> bool {
        browser.apply_scan_discovered_batch(batch)
    }

    pub(in crate::native_app) fn plan_filesystem_change(
        &mut self,
        browser: &mut FolderBrowserState,
        source_id: String,
        paths: &[PathBuf],
        overflowed: bool,
    ) -> SourceFilesystemChangePlan {
        if browser.source_root_path(&source_id).is_none() {
            self.pending_refreshes.remove(&source_id);
            return SourceFilesystemChangePlan::IgnoredSourceMissing { source_id };
        }
        if !overflowed && !paths.is_empty() {
            let changed = browser.refresh_filesystem_paths(&source_id, paths);
            return SourceFilesystemChangePlan::Patched {
                source_id,
                changed_count: paths.len(),
                changed,
            };
        }
        if self.active() {
            self.pending_refreshes.insert(source_id.clone());
            return SourceFilesystemChangePlan::DeferredAlreadyRunning { source_id };
        }
        SourceFilesystemChangePlan::QueueRefresh { source_id }
    }

    pub(in crate::native_app) fn next_pending_refresh_if_idle(&mut self) -> Option<String> {
        if self.active() {
            return None;
        }
        let source_id = self.pending_refreshes.iter().next().cloned()?;
        self.pending_refreshes.remove(&source_id);
        Some(source_id)
    }

    pub(in crate::native_app) fn begin_filesystem_refresh(
        &mut self,
        browser: &mut FolderBrowserState,
        source_id: String,
        task_id: u64,
    ) -> SourceRefreshRequest {
        if let Some(request) = browser.begin_source_scan(source_id.clone(), task_id) {
            return SourceRefreshRequest::Queued(request);
        }
        self.pending_refreshes.insert(source_id.clone());
        SourceRefreshRequest::Deferred { source_id }
    }

    pub(in crate::native_app) fn finish_scan(
        &mut self,
        browser: &mut FolderBrowserState,
        result: FolderScanResult,
    ) -> SourceScanFinish {
        let source_id = result.source_id.clone();
        let label = result.label.clone();
        let file_count = result.file_count;
        let folder_count = result.folder_count;
        let source_db_error = result.source_db_error.clone();
        if browser.apply_scan_finished(result) {
            self.progress = None;
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
                source_db_error,
            }
        } else {
            SourceScanFinish::Stale { label }
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn set_progress_for_tests(&mut self, progress: FolderScanProgress) {
        self.progress = Some(progress);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn pending_refresh_contains_for_tests(
        &self,
        source_id: &str,
    ) -> bool {
        self.pending_refreshes.contains(source_id)
    }
}
