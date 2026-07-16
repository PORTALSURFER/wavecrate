use std::{collections::VecDeque, path::PathBuf};

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
    pending_refreshes: VecDeque<PendingSourceRefresh>,
}

struct PendingSourceRefresh {
    source_id: String,
    selection_requested: bool,
    scan_required: bool,
}

pub(in crate::native_app) enum SourceFilesystemChangePlan {
    IgnoredSourceMissing {
        source_id: String,
    },
    SyncPaths {
        source_id: String,
        changed_count: usize,
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
    IgnoredMissing { source_id: String },
}

pub(in crate::native_app) enum SourceSelectionRequest {
    Queued(FolderScanRequest),
    Deferred,
    Settled,
}

pub(in crate::native_app) enum SourceScanFinish {
    Applied {
        source_id: String,
        label: String,
        file_count: usize,
        folder_count: usize,
        source_db_error: Option<String>,
        source_root_available: bool,
    },
    Stale {
        label: String,
    },
    Cancelled {
        source_id: String,
        label: String,
    },
}

impl SourceScanWorkflow {
    pub(in crate::native_app) fn new() -> Self {
        Self {
            progress: None,
            pending_refreshes: VecDeque::new(),
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
        if self.active() {
            if let Some(source_id) = browser.source_id_for_root_path(&root) {
                let _ = self.begin_select_source(browser, source_id, task_id);
            } else if let Some(source_id) = browser.defer_add_source_path(root, true) {
                self.queue_selected_required_refresh(source_id);
            }
            return None;
        }
        browser.begin_add_source_path(root, task_id)
    }

    pub(in crate::native_app) fn begin_add_source_path_preserving_selection(
        &mut self,
        browser: &mut FolderBrowserState,
        root: PathBuf,
        task_id: u64,
    ) -> Option<FolderScanRequest> {
        if self.active() {
            if let Some(source_id) = browser.source_id_for_root_path(&root)
                && !browser.source_is_missing(&source_id)
            {
                self.queue_required_refresh(source_id);
            } else if let Some(source_id) = browser.defer_add_source_path(root, false) {
                self.queue_required_refresh(source_id);
            }
            return None;
        }
        browser.begin_add_source_path_preserving_selection(root, task_id)
    }

    pub(in crate::native_app) fn begin_select_source(
        &mut self,
        browser: &mut FolderBrowserState,
        id: String,
        task_id: u64,
    ) -> SourceSelectionRequest {
        if self.active() {
            let active_source_id = self
                .progress
                .as_ref()
                .map(|progress| progress.source_id.as_str());
            if active_source_id == Some(id.as_str()) {
                let _ = browser.select_source_without_scan(id.clone());
                self.clear_pending_selection(&id);
                return SourceSelectionRequest::Settled;
            }
            if !browser.select_source_without_scan(id.clone()) {
                return SourceSelectionRequest::Settled;
            }
            if browser.source_is_missing(&id) {
                self.remove_pending_refresh(&id);
                return SourceSelectionRequest::Settled;
            }
            self.queue_pending_selection(id);
            return if browser.selected_source_loaded() {
                SourceSelectionRequest::Settled
            } else {
                SourceSelectionRequest::Deferred
            };
        }
        browser
            .begin_select_source(id, task_id)
            .map(SourceSelectionRequest::Queued)
            .unwrap_or(SourceSelectionRequest::Settled)
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
        source_root_available: bool,
    ) -> SourceFilesystemChangePlan {
        let Some(source_missing) =
            browser.apply_observed_source_availability(&source_id, source_root_available)
        else {
            self.remove_pending_refresh(&source_id);
            return SourceFilesystemChangePlan::IgnoredSourceMissing { source_id };
        };
        if source_missing {
            self.remove_pending_refresh(&source_id);
            return SourceFilesystemChangePlan::IgnoredSourceMissing { source_id };
        }
        if !overflowed && !paths.is_empty() {
            return SourceFilesystemChangePlan::SyncPaths {
                source_id,
                changed_count: paths.len(),
            };
        }
        if self.active() {
            self.queue_required_refresh(source_id.clone());
            return SourceFilesystemChangePlan::DeferredAlreadyRunning { source_id };
        }
        SourceFilesystemChangePlan::QueueRefresh { source_id }
    }

    pub(in crate::native_app) fn next_pending_refresh_if_idle(&mut self) -> Option<String> {
        if self.active() {
            return None;
        }
        let pending_selection = self
            .pending_refreshes
            .iter()
            .rposition(|pending| pending.selection_requested);
        pending_selection
            .and_then(|index| self.pending_refreshes.remove(index))
            .or_else(|| self.pending_refreshes.pop_back())
            .map(|pending| pending.source_id)
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
        if !browser.source_exists(&source_id) || browser.source_is_missing(&source_id) {
            self.remove_pending_refresh(&source_id);
            return SourceRefreshRequest::IgnoredMissing { source_id };
        }
        self.queue_required_refresh(source_id.clone());
        SourceRefreshRequest::Deferred { source_id }
    }

    fn queue_pending_selection(&mut self, source_id: String) {
        self.queue_pending_refresh(source_id, true, false);
    }

    fn queue_required_refresh(&mut self, source_id: String) {
        self.queue_pending_refresh(source_id, false, true);
    }

    fn queue_selected_required_refresh(&mut self, source_id: String) {
        self.queue_pending_refresh(source_id, true, true);
    }

    fn queue_pending_refresh(
        &mut self,
        source_id: String,
        selection_requested: bool,
        scan_required: bool,
    ) {
        let previous = self
            .pending_refreshes
            .iter()
            .find(|pending| pending.source_id == source_id)
            .map(|pending| (pending.selection_requested, pending.scan_required))
            .unwrap_or_default();
        self.remove_pending_refresh(&source_id);
        self.pending_refreshes.push_back(PendingSourceRefresh {
            source_id,
            selection_requested: previous.0 || selection_requested,
            scan_required: previous.1 || scan_required,
        });
    }

    fn clear_pending_selection(&mut self, source_id: &str) {
        if let Some(pending) = self
            .pending_refreshes
            .iter_mut()
            .find(|pending| pending.source_id == source_id)
        {
            pending.selection_requested = false;
        }
        self.pending_refreshes
            .retain(|pending| pending.selection_requested || pending.scan_required);
    }

    fn remove_pending_refresh(&mut self, source_id: &str) {
        self.pending_refreshes
            .retain(|pending| pending.source_id != source_id);
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
        let source_root_available = result.source_root_available;
        if result.cancelled {
            if browser.cancel_scan(&source_id, result.task_id) {
                self.progress = None;
                self.queue_required_refresh(source_id.clone());
                return SourceScanFinish::Cancelled { source_id, label };
            }
            return SourceScanFinish::Stale { label };
        }
        if browser.apply_scan_finished(result) {
            self.progress = None;
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
                source_db_error,
                source_root_available,
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
        self.pending_refreshes
            .iter()
            .any(|pending| pending.source_id == source_id)
    }
}
