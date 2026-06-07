use super::*;
use crate::app::controller::source_watcher::SourceWatchCause;
use crate::sample_sources::scanner::ScanMode;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::{Duration, Instant};

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

    fn request_auto_watcher_sync_for_source_if_due(
        &mut self,
        source_id: &SourceId,
        paths: Vec<PathBuf>,
        overflowed: bool,
        min_interval: Duration,
    ) {
        if overflowed || paths.is_empty() {
            self.request_auto_quick_sync_for_source_if_due(source_id, min_interval);
            return;
        }
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
        else {
            return;
        };
        self.request_auto_targeted_sync_for_source(source, paths, min_interval);
    }

    /// Trigger a quick sync for a specific source when the debounce interval elapses.
    pub(crate) fn request_auto_quick_sync_for_source_if_due(
        &mut self,
        source_id: &SourceId,
        min_interval: Duration,
    ) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
        else {
            return;
        };
        self.request_auto_quick_sync_for_source(source, min_interval);
    }

    fn request_auto_quick_sync_for_source(&mut self, source: SampleSource, min_interval: Duration) {
        self.request_auto_scan_for_source(source, ScanMode::Quick, min_interval, None);
    }

    fn request_auto_targeted_sync_for_source(
        &mut self,
        source: SampleSource,
        paths: Vec<PathBuf>,
        min_interval: Duration,
    ) {
        self.request_auto_scan_for_source(source, ScanMode::Targeted, min_interval, Some(paths));
    }

    fn request_auto_scan_for_source(
        &mut self,
        source: SampleSource,
        mode: ScanMode,
        min_interval: Duration,
        paths: Option<Vec<PathBuf>>,
    ) {
        if self.runtime.jobs.scan_in_progress() {
            return;
        }
        if self.library.missing.sources.contains(&source.id) {
            return;
        }
        if self.source_has_pending_file_mutations(&source.id) {
            return;
        }
        let now = Instant::now();
        if self.source_auto_sync_grace_active(&source.id, now) {
            return;
        }
        let last_sync = self
            .runtime
            .auto_sync_last_by_source
            .get(&source.id)
            .copied();
        if !auto_sync_due(last_sync, now, min_interval) {
            return;
        }
        self.runtime
            .auto_sync_last_by_source
            .insert(source.id.clone(), now);
        self.request_scan_for_source_with_paths(&source, mode, ScanKind::Auto, paths);
    }

    fn request_scan_with_mode(&mut self, mode: ScanMode, kind: ScanKind) {
        let Some(source) = self.current_source() else {
            self.set_status_message(StatusMessage::SelectSourceToScan);
            return;
        };
        self.request_scan_for_source_with_paths(&source, mode, kind, None);
    }

    fn request_scan_for_source(&mut self, source: &SampleSource, mode: ScanMode, kind: ScanKind) {
        self.request_scan_for_source_with_paths(source, mode, kind, None);
    }

    fn request_scan_for_source_with_paths(
        &mut self,
        source: &SampleSource,
        mode: ScanMode,
        kind: ScanKind,
        paths: Option<Vec<PathBuf>>,
    ) {
        if self.runtime.jobs.scan_in_progress() {
            if matches!(kind, ScanKind::Manual) {
                self.set_status_message(StatusMessage::ScanAlreadyRunning);
            }
            return;
        }
        self.prepare_for_scan(source, mode);
        if matches!(kind, ScanKind::Manual) {
            self.begin_scan_progress(mode, source);
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();
        self.runtime.jobs.start_scan(rx, cancel.clone());
        let source_id = source.id.clone();
        let root = source.root.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<
                crate::sample_sources::scanner::ScanStats,
                crate::sample_sources::scanner::ScanError,
            > {
                let db = SourceDatabase::open_fast(&root)?;
                let mut progress = |completed, path: &std::path::Path| {
                    if completed == 1 || completed % 128 == 0 {
                        let _ = tx.send(ScanJobMessage::Progress {
                            completed,
                            detail: Some(path.display().to_string()),
                        });
                    }
                };
                let stats = if mode == ScanMode::Targeted {
                    let paths = paths.unwrap_or_default();
                    crate::sample_sources::scanner::sync_paths_with_progress(
                        &db,
                        &paths,
                        Some(cancel.as_ref()),
                        &mut progress,
                    )?
                } else {
                    crate::sample_sources::scanner::scan_with_progress(
                        &db,
                        mode,
                        Some(cancel.as_ref()),
                        &mut progress,
                    )?
                };
                if stats.hashes_pending > 0 {
                    crate::sample_sources::scanner::schedule_deep_hash_scan(root.clone());
                }
                Ok(stats)
            })();
            let _ = tx.send(ScanJobMessage::Finished(ScanResult {
                source_id,
                mode,
                kind,
                result,
            }));
        });
    }

    fn prepare_for_scan(&mut self, source: &SampleSource, mode: ScanMode) {
        if matches!(mode, ScanMode::Hard) {
            let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
                &mut self.cache,
                &mut self.ui_cache,
                &mut self.library.missing,
            );
            invalidator.invalidate_wav_related(&source.id);
        }
    }
}

fn auto_sync_due(last_sync: Option<Instant>, now: Instant, min_interval: Duration) -> bool {
    last_sync.is_none_or(|last| now.saturating_duration_since(last) >= min_interval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::jobs::JobMessage;
    use crate::app::controller::test_support::dummy_controller;
    use std::path::PathBuf;

    #[test]
    fn auto_sync_due_respects_interval() {
        let now = Instant::now();
        assert!(auto_sync_due(None, now, Duration::from_secs(5)));
        assert!(!auto_sync_due(
            Some(now),
            now + Duration::from_secs(3),
            Duration::from_secs(5)
        ));
        assert!(auto_sync_due(
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
