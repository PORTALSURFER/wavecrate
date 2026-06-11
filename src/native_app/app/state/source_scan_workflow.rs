use std::{collections::HashSet, path::PathBuf};

use crate::native_app::{
    app::GuiMessage,
    sample_library::folder_browser::{
        FolderBrowserState,
        scan::{
            self, FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanProgress,
            FolderScanRequest, FolderScanResult,
        },
    },
};

const DISCOVERY_BATCH_SIZE: usize = 64;

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
    },
    Stale {
        label: String,
    },
}

pub(in crate::native_app) fn run_folder_scan_worker(
    request: FolderScanRequest,
    sender: std::sync::mpsc::Sender<GuiMessage>,
) -> FolderScanResult {
    let discovery_sender = sender.clone();
    let mut pending_discoveries = Vec::with_capacity(DISCOVERY_BATCH_SIZE);
    let task_id = request.task_id;
    let source_id = request.source_id.clone();
    let result = scan::scan_source_with_progress(
        request,
        |progress| {
            let _ = sender.send(GuiMessage::FolderScanProgress(progress));
        },
        |event| {
            pending_discoveries.push(event);
            if pending_discoveries.len() >= DISCOVERY_BATCH_SIZE {
                send_discovery_batch(
                    &discovery_sender,
                    task_id,
                    source_id.clone(),
                    &mut pending_discoveries,
                );
            }
        },
    );
    if !pending_discoveries.is_empty() {
        send_discovery_batch(
            &discovery_sender,
            task_id,
            source_id,
            &mut pending_discoveries,
        );
    }
    result
}

fn send_discovery_batch(
    sender: &std::sync::mpsc::Sender<GuiMessage>,
    task_id: u64,
    source_id: String,
    pending_discoveries: &mut Vec<FolderScanDiscovery>,
) {
    let events = std::mem::take(pending_discoveries);
    let _ = sender.send(GuiMessage::FolderScanDiscoveryBatch(
        FolderScanDiscoveryBatch {
            task_id,
            source_id,
            events,
        },
    ));
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
        if browser.apply_scan_finished(result) {
            self.progress = None;
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::sample_library::folder_browser::scan::{
        FolderScanProgress, FolderScanRequest, scan_source_with_progress,
    };
    use std::fs;

    fn temp_dir_with_wav() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("source root");
        fs::write(root.path().join("sample.wav"), [0_u8; 8]).expect("write sample");
        root
    }

    #[test]
    fn stale_progress_is_ignored() {
        let root = temp_dir_with_wav();
        let mut browser = FolderBrowserState::load_default();
        let mut workflow = SourceScanWorkflow::new();
        let request = workflow
            .begin_add_source_path(&mut browser, root.path().to_path_buf(), 7)
            .expect("scan request");
        workflow.start_scan(&request);

        let stale = FolderScanProgress {
            task_id: request.task_id + 1,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Scanning"),
            completed: 1,
            total: 1,
            detail: String::new(),
        };

        assert!(!workflow.apply_progress(&browser, stale));
        assert_eq!(
            workflow.progress().expect("queued progress").phase,
            "Queued"
        );
    }

    #[test]
    fn stale_finish_keeps_active_scan_owner() {
        let root = temp_dir_with_wav();
        let mut browser = FolderBrowserState::load_default();
        let mut workflow = SourceScanWorkflow::new();
        let request = workflow
            .begin_add_source_path(&mut browser, root.path().to_path_buf(), 11)
            .expect("scan request");
        workflow.start_scan(&request);
        let stale_result = scan_source_with_progress(
            FolderScanRequest {
                task_id: request.task_id + 1,
                source_id: request.source_id.clone(),
                label: request.label.clone(),
                root: request.root.clone(),
            },
            |_| {},
            |_| {},
        );

        assert!(matches!(
            workflow.finish_scan(&mut browser, stale_result),
            SourceScanFinish::Stale { .. }
        ));
        assert!(workflow.active());
    }

    #[test]
    fn pending_refresh_waits_for_active_scan() {
        let root = temp_dir_with_wav();
        let mut browser = FolderBrowserState::load_default();
        let mut workflow = SourceScanWorkflow::new();
        let request = workflow
            .begin_add_source_path(&mut browser, root.path().to_path_buf(), 21)
            .expect("scan request");
        let source_id = request.source_id.clone();
        workflow.start_scan(&request);

        let plan = workflow.plan_filesystem_change(&mut browser, source_id.clone(), &[], true);

        assert!(matches!(
            plan,
            SourceFilesystemChangePlan::DeferredAlreadyRunning { .. }
        ));
        assert_eq!(workflow.next_pending_refresh_if_idle(), None);
        let result = scan_source_with_progress(request, |_| {}, |_| {});
        assert!(matches!(
            workflow.finish_scan(&mut browser, result),
            SourceScanFinish::Applied { .. }
        ));
        assert_eq!(workflow.next_pending_refresh_if_idle(), Some(source_id));
    }
}
