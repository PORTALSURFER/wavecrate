use super::*;
use crate::app::controller::source_watcher::SourceWatchCause;
use crate::sample_sources::scanner::ScanMode;
use std::path::PathBuf;
use std::time::Duration;

mod request_policy;
mod watcher_sync;
mod worker;

const WATCHER_SYNC_INTERVAL: Duration = Duration::from_secs(2);

impl AppController {
    /// Trigger a quick sync (incremental scan) of the selected source.
    pub fn request_quick_sync(&mut self) {
        self.request_scan_with_mode(ScanMode::Quick, ScanKind::Manual);
    }

    /// Trigger a hard sync (full rescan that prunes missing rows) of the selected source.
    pub fn request_hard_sync(&mut self) {
        self.request_scan_with_mode(ScanMode::Hard, ScanKind::Manual);
    }

    /// Trigger a hard sync for a specific source by id when follow-up recovery needs it.
    pub(crate) fn request_hard_sync_for_source(&mut self, source_id: &SourceId) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
        else {
            return;
        };
        self.request_scan_for_source(&source, ScanMode::Hard, ScanKind::Manual);
    }

    /// Trigger a quick sync for a specific source by id.
    pub(crate) fn request_quick_sync_for_source(&mut self, source_id: &SourceId) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
        else {
            return;
        };
        self.request_scan_for_source(&source, ScanMode::Quick, ScanKind::Manual);
    }

    /// Trigger a periodic quick sync to keep the current source in sync with disk.
    pub(crate) fn request_auto_quick_sync_if_due(&mut self, min_interval: Duration) {
        let Some(source) = self.current_source() else {
            return;
        };
        self.request_auto_quick_sync_for_source_if_due(&source.id, min_interval);
    }

    /// Trigger a quick sync for a source based on a file watcher event.
    pub(crate) fn handle_source_watch_event(
        &mut self,
        source_id: &SourceId,
        cause: SourceWatchCause,
        paths: Vec<PathBuf>,
        overflowed: bool,
    ) {
        match cause {
            SourceWatchCause::ExternalFileChange => {
                self.request_auto_watcher_sync_for_source_if_due(
                    source_id,
                    paths,
                    overflowed,
                    WATCHER_SYNC_INTERVAL,
                );
            }
            SourceWatchCause::ControllerFileOp => {
                tracing::debug!(
                    source_id = %source_id,
                    "source watch event matched controller file-op paths"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::jobs::JobMessage;
    use crate::app::controller::test_support::dummy_controller;
    use std::path::PathBuf;

    #[test]
    fn auto_sync_due_respects_interval() {
        let now = std::time::Instant::now();
        assert!(request_policy::auto_sync_due(
            None,
            now,
            Duration::from_secs(5)
        ));
        assert!(!request_policy::auto_sync_due(
            Some(now),
            now + Duration::from_secs(3),
            Duration::from_secs(5)
        ));
        assert!(request_policy::auto_sync_due(
            Some(now),
            now + Duration::from_secs(6),
            Duration::from_secs(5)
        ));
    }

    #[test]
    fn auto_quick_sync_skips_sources_with_pending_file_mutations() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.begin_pending_file_mutation(&source.id, [PathBuf::from("kick.wav")]);

        controller.request_auto_quick_sync_for_source_if_due(&source.id, Duration::from_secs(0));

        assert!(!controller.runtime.jobs.scan_in_progress());
    }

    #[test]
    fn manual_scan_is_blocked_while_source_remap_is_pending() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.runtime.source_lane.pending_remap =
            Some(crate::app::controller::state::runtime::PendingSourceRemap {
                request_id: 44,
                source: source.clone(),
                new_root: tempfile::tempdir().expect("destination").keep(),
                queued_at: std::time::Instant::now(),
                canceled: false,
                write_fence: std::sync::Arc::new(
                    crate::app::controller::jobs::SourceRemapWriteFence::default(),
                ),
            });

        controller.request_quick_sync_for_source(&source.id);

        assert!(!controller.runtime.jobs.scan_in_progress());
        assert_eq!(controller.ui.status.text, "Source remap in progress");
    }

    #[test]
    fn canceled_source_remap_does_not_block_manual_scan() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.runtime.source_lane.pending_remap =
            Some(crate::app::controller::state::runtime::PendingSourceRemap {
                request_id: 45,
                source: source.clone(),
                new_root: tempfile::tempdir().expect("destination").keep(),
                queued_at: std::time::Instant::now(),
                canceled: true,
                write_fence: std::sync::Arc::new(
                    crate::app::controller::jobs::SourceRemapWriteFence::default(),
                ),
            });

        controller.request_quick_sync_for_source(&source.id);

        assert!(controller.runtime.jobs.scan_in_progress());
    }

    #[test]
    fn canceled_source_remap_does_not_block_auto_scan() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.runtime.source_lane.pending_remap =
            Some(crate::app::controller::state::runtime::PendingSourceRemap {
                request_id: 46,
                source: source.clone(),
                new_root: tempfile::tempdir().expect("destination").keep(),
                queued_at: std::time::Instant::now(),
                canceled: true,
                write_fence: std::sync::Arc::new(
                    crate::app::controller::jobs::SourceRemapWriteFence::default(),
                ),
            });

        controller.request_auto_quick_sync_for_source_if_due(&source.id, Duration::from_secs(0));

        assert!(controller.runtime.jobs.scan_in_progress());
        assert!(
            controller
                .runtime
                .source_sync
                .auto_sync_last_by_source
                .contains_key(&source.id)
        );
    }

    #[test]
    fn auto_quick_sync_respects_recent_internal_file_mutation_grace() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.begin_pending_file_mutation(&source.id, [PathBuf::from("kick.wav")]);
        controller.finish_pending_file_mutation(&source.id, [PathBuf::from("kick.wav")]);

        controller.request_auto_quick_sync_for_source_if_due(&source.id, Duration::from_secs(0));

        assert!(!controller.runtime.jobs.scan_in_progress());
    }

    #[test]
    fn controller_file_op_source_watch_event_does_not_launch_auto_sync() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());

        controller.handle_source_watch_event(
            &source.id,
            SourceWatchCause::ControllerFileOp,
            Vec::new(),
            false,
        );

        assert!(!controller.runtime.jobs.scan_in_progress());
    }

    #[test]
    fn external_source_watch_paths_launch_targeted_sync() {
        let temp = tempfile::tempdir().expect("temp source");
        std::fs::write(temp.path().join("kick.wav"), b"kick").expect("write sample");
        let (mut controller, mut source) = dummy_controller();
        source.root = temp.path().to_path_buf();
        controller.library.sources.push(source.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());

        controller.handle_source_watch_event(
            &source.id,
            SourceWatchCause::ExternalFileChange,
            vec![PathBuf::from("kick.wav")],
            false,
        );

        assert!(controller.runtime.jobs.scan_in_progress());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let message = loop {
            match controller.runtime.jobs.try_recv_message() {
                Ok(JobMessage::Scan(ScanJobMessage::Finished(result))) => break result,
                Ok(_) => continue,
                Err(err) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    if matches!(err, std::sync::mpsc::TryRecvError::Disconnected) {
                        panic!("expected targeted scan completion: {err}");
                    }
                }
                Err(err) => panic!("expected targeted scan completion: {err}"),
            }
        };
        assert_eq!(message.mode, ScanMode::Targeted);
        let stats = message.result.expect("targeted sync should finish");
        assert_eq!(stats.added, 1);
        assert_eq!(stats.total_files, 1);
    }
}
