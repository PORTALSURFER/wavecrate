//! Folder/source scan lifecycle dispatch helpers.

use super::*;

impl ControllerJobs {
    /// Return whether a source scan job is currently running.
    pub(in super::super::super) fn scan_in_progress(&self) -> bool {
        self.in_progress.scan
    }

    /// Return the source id currently being scanned for folders, if any.
    pub(in super::super::super) fn pending_folder_scan_source(&self) -> Option<SourceId> {
        self.pending_folder_scan
            .as_ref()
            .map(|pending| pending.source_id.clone())
    }

    /// Start a background scan for folders under `root`, canceling any in-flight scan.
    pub(in super::super::super) fn request_folder_scan(
        &mut self,
        source_id: SourceId,
        root: PathBuf,
    ) -> u64 {
        if let Some(cancel) = self.cancel_handles.folder_scan.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        let request_id = self.request_counters.next_folder_scan_request_id;
        self.request_counters.next_folder_scan_request_id = self
            .request_counters
            .next_folder_scan_request_id
            .wrapping_add(1)
            .max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_handles.folder_scan = Some(cancel.clone());
        self.pending_folder_scan = Some(PendingFolderScan {
            request_id,
            source_id: source_id.clone(),
        });
        self.spawn_optional_one_shot_job(true, move || {
            let folders = crate::app::controller::library::source_folders::scan_disk_folders(
                &root,
                cancel.as_ref(),
            );
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            Some(JobMessage::FolderScanFinished(FolderScanResult {
                request_id,
                source_id,
                folders,
            }))
        });
        request_id
    }

    /// Clear folder scan tracking state after a scan completes.
    pub(in super::super::super) fn clear_folder_scan(&mut self) {
        self.cancel_handles.folder_scan = None;
        self.pending_folder_scan = None;
    }

    /// Return whether a folder scan result matches the latest request.
    pub(in super::super::super) fn folder_scan_matches(
        &self,
        request_id: u64,
        source_id: &SourceId,
    ) -> bool {
        self.pending_folder_scan.as_ref().is_some_and(|pending| {
            pending.request_id == request_id && &pending.source_id == source_id
        })
    }

    /// Start forwarding stream updates for a source scan operation.
    pub(in super::super::super) fn start_scan(
        &mut self,
        rx: Receiver<ScanJobMessage>,
        cancel: Arc<AtomicBool>,
    ) {
        self.in_progress.scan = true;
        self.cancel_handles.scan = Some(cancel);
        self.send_source_watch_scan_state(true);
        self.start_progress_stream(rx, JobMessage::Scan, scan_message_is_finished);
    }

    /// Return the cooperative cancel handle for the active source scan.
    pub(in super::super::super) fn scan_cancel(&self) -> Option<Arc<AtomicBool>> {
        self.cancel_handles.scan.clone()
    }

    /// Clear scan in-progress state and notify the source watcher.
    pub(in super::super::super) fn clear_scan(&mut self) {
        self.in_progress.scan = false;
        self.cancel_handles.scan = None;
        self.send_source_watch_scan_state(false);
    }
}

fn scan_message_is_finished(message: &ScanJobMessage) -> bool {
    matches!(message, ScanJobMessage::Finished(_))
}
