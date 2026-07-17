use std::time::Instant;

use crate::native_app::{
    app::{NativeAppState, emit_gui_action, logging},
    sample_library::folder_browser::scan::{FolderScanDiscoveryBatch, FolderScanProgress},
};

impl NativeAppState {
    pub(in crate::native_app) fn apply_folder_scan_progress(
        &mut self,
        progress: FolderScanProgress,
    ) {
        let started_at = Instant::now();
        let label = progress.label.clone();
        let phase = progress.phase.clone();
        if self.library.apply_folder_scan_progress(progress) {
            self.ui.status.sample = format!("{phase} source {label}");
            emit_gui_action(
                "folder_browser.scan.progress",
                Some("folder_browser"),
                Some(&phase),
                "active",
                started_at,
                None,
            );
        }
    }

    pub(in crate::native_app) fn apply_folder_scan_discovery_batch(
        &mut self,
        batch: FolderScanDiscoveryBatch,
    ) {
        let started_at = Instant::now();
        let count = batch.events.len();
        self.library.apply_folder_scan_discovery_batch(batch);
        if logging::debug_logging_enabled() {
            tracing::debug!(
                target: logging::ACTION_EVENT_TARGET,
                event = "action_detail",
                action = "folder_browser.scan.discovery_batch",
                pane = "folder_browser",
                item_count = count,
                "Folder browser scan discovery batch applied"
            );
        }
        emit_gui_action(
            "folder_browser.scan.discovery_batch",
            Some("folder_browser"),
            None,
            "applied",
            started_at,
            None,
        );
    }
}
