use super::*;
use crate::sample_sources::scanner::ScanMode;
use std::path::PathBuf;
use std::time::{Duration, Instant};

impl AppController {
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

    pub(super) fn request_auto_targeted_sync_for_source(
        &mut self,
        source: SampleSource,
        paths: Vec<PathBuf>,
        min_interval: Duration,
    ) {
        self.request_auto_scan_for_source(source, ScanMode::Targeted, min_interval, Some(paths));
    }

    pub(super) fn request_scan_with_mode(&mut self, mode: ScanMode, kind: ScanKind) {
        let Some(source) = self.current_source() else {
            self.set_status_message(StatusMessage::SelectSourceToScan);
            return;
        };
        self.request_scan_for_source_with_paths(&source, mode, kind, None);
    }

    pub(super) fn request_scan_for_source(
        &mut self,
        source: &SampleSource,
        mode: ScanMode,
        kind: ScanKind,
    ) {
        self.request_scan_for_source_with_paths(source, mode, kind, None);
    }

    fn request_auto_quick_sync_for_source(&mut self, source: SampleSource, min_interval: Duration) {
        self.request_auto_scan_for_source(source, ScanMode::Quick, min_interval, None);
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
            .source_sync
            .auto_sync_last_by_source
            .get(&source.id)
            .copied();
        if !auto_sync_due(last_sync, now, min_interval) {
            return;
        }
        self.runtime
            .source_sync
            .auto_sync_last_by_source
            .insert(source.id.clone(), now);
        self.request_scan_for_source_with_paths(&source, mode, ScanKind::Auto, paths);
    }

    fn request_scan_for_source_with_paths(
        &mut self,
        source: &SampleSource,
        mode: ScanMode,
        kind: ScanKind,
        paths: Option<Vec<PathBuf>>,
    ) {
        if self
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| pending.source.id == source.id)
        {
            if matches!(kind, ScanKind::Manual) {
                self.set_status("Source remap in progress", StatusTone::Info);
            }
            return;
        }
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
        worker::launch_scan_worker(self, source, mode, kind, paths);
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

pub(super) fn auto_sync_due(
    last_sync: Option<Instant>,
    now: Instant,
    min_interval: Duration,
) -> bool {
    last_sync.is_none_or(|last| now.saturating_duration_since(last) >= min_interval)
}
