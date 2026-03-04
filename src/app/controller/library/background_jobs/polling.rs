use super::*;
use crate::app::controller::jobs::{
    AnalysisFailuresResult, FolderScanResult, NormalizationResult, SearchResult,
    SourceDbMaintenanceResult, UmapBuildResult, UmapClusterBuildResult,
};
use crate::app::controller::playback::recording::waveform_loader::RecordingWaveformLoadResult;
use crate::app::controller::state::audio::{PendingAudio, PendingRecordingWaveform};
use std::path::Path;

impl AppController {
    pub(crate) fn poll_background_jobs(&mut self) {
        self.apply_progress_cancel_request();
        while let Some(message) = self.try_next_background_job_message() {
            self.handle_background_job_message(message);
        }
    }

    fn apply_progress_cancel_request(&mut self) {
        if !self.ui.progress.cancel_requested {
            return;
        }
        match cancel_request_action(self.ui.progress.task) {
            CancelRequestAction::TrashMove => {
                if let Some(cancel) = self.runtime.jobs.trash_move_cancel().as_ref() {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            CancelRequestAction::Scan => {
                if let Some(cancel) = self.runtime.jobs.scan_cancel().as_ref() {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            CancelRequestAction::Analysis => {
                self.runtime.analysis.cancel();
                self.clear_progress();
            }
            CancelRequestAction::FileOps => {
                if let Some(cancel) = self.runtime.jobs.file_ops_cancel().as_ref() {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            CancelRequestAction::None => {}
        }
    }

    fn try_next_background_job_message(&self) -> Option<JobMessage> {
        self.runtime.jobs.try_recv_message().ok()
    }

    fn handle_background_job_message(&mut self, message: JobMessage) {
        match message {
            JobMessage::WavLoaded(message) => self.handle_wav_loaded_message(message),
            JobMessage::AudioLoaded(message) => self.handle_audio_loaded_message(message),
            JobMessage::RecordingWaveformLoaded(message) => {
                self.handle_recording_waveform_loaded_message(message)
            }
            JobMessage::Scan(message) => self.handle_scan_message(message),
            JobMessage::FolderScanFinished(message) => {
                self.handle_folder_scan_finished_message(message)
            }
            JobMessage::SourceWatch(message) => {
                self.handle_source_watch_event(&message.source_id);
            }
            JobMessage::TrashMove(message) => self.handle_trash_move_message(message),
            JobMessage::FolderDeleteRecoveryFinished(report) => {
                self.apply_folder_delete_recovery_report(report);
            }
            JobMessage::FileOps(message) => self.handle_file_ops_message(message),
            JobMessage::Analysis(message) => {
                analysis::handle_analysis_message(self, message);
            }
            JobMessage::AnalysisFailuresLoaded(message) => {
                self.handle_analysis_failures_loaded_message(message)
            }
            JobMessage::UmapBuilt(message) => self.handle_umap_built_message(message),
            JobMessage::UmapClustersBuilt(message) => {
                self.handle_umap_clusters_built_message(message)
            }
            JobMessage::SimilarityPrepared(message) => {
                similarity::handle_similarity_prepared(self, message);
            }
            JobMessage::UpdateChecked(message) => {
                updates::handle_update_checked(self, message);
            }
            JobMessage::IssueGatewayCreated(message) => {
                updates::handle_issue_gateway_created(self, message);
            }
            JobMessage::IssueGatewayAuthed(message) => {
                updates::handle_issue_gateway_authed(self, message);
            }
            JobMessage::IssueTokenLoaded(message) => {
                updates::handle_issue_token_loaded(self, message);
            }
            JobMessage::IssueTokenSaved(message) => {
                updates::handle_issue_token_saved(self, message);
            }
            JobMessage::IssueTokenDeleted(message) => {
                updates::handle_issue_token_deleted(self, message);
            }
            JobMessage::SourceDbMaintenanceFinished(message) => {
                self.handle_source_db_maintenance_finished_message(message)
            }
            JobMessage::BrowserSearchFinished(message) => {
                self.handle_browser_search_finished_message(message)
            }
            JobMessage::Normalized(message) => self.handle_normalized_message(message),
        }
    }

    fn handle_wav_loaded_message(&mut self, message: WavLoadResult) {
        if Some(&message.source_id) != self.selection_state.ctx.selected_source.as_ref() {
            return;
        }
        match message.result {
            Ok(entries) => {
                self.apply_wav_entries(
                    entries,
                    message.total,
                    self.wav_entries.page_size,
                    message.page_index,
                    false,
                    Some(message.source_id.clone()),
                    Some(message.elapsed),
                );
                self.cache.wav.insert_page(
                    message.source_id.clone(),
                    message.total,
                    self.wav_entries.page_size,
                    message.page_index,
                    self.wav_entries
                        .pages
                        .get(&message.page_index)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            Err(err) => self.handle_wav_load_error(&message.source_id, err),
        }
        self.runtime.jobs.clear_wav_load_pending();
        if self.ui.progress.task == Some(ProgressTaskKind::WavLoad) {
            self.clear_progress();
        }
    }

    fn handle_audio_loaded_message(&mut self, message: AudioLoadResult) {
        match message {
            AudioLoadResult::Primary {
                request_id,
                source_id,
                relative_path,
                result,
            } => {
                let Some(pending) = self.runtime.jobs.pending_audio() else {
                    return;
                };
                if !pending_audio_matches(&pending, request_id, &source_id, &relative_path) {
                    return;
                }
                self.runtime.jobs.set_pending_audio(None);
                self.ui.waveform.loading = None;
                match result {
                    Ok(outcome) => self.handle_audio_loaded(pending, outcome),
                    Err(err) => self.handle_audio_load_error(pending, err),
                }
            }
            AudioLoadResult::Transients(result) => {
                self.handle_audio_transients_loaded(result);
            }
        }
    }

    fn handle_recording_waveform_loaded_message(&mut self, message: RecordingWaveformLoadResult) {
        let Some(pending) = self.runtime.jobs.pending_recording_waveform() else {
            return;
        };
        if !pending_recording_waveform_matches(
            &pending,
            message.request_id,
            &message.source_id,
            &message.relative_path,
        ) {
            return;
        }
        self.runtime.jobs.set_pending_recording_waveform(None);
        let target_matches = match self.audio.recording_target.as_ref() {
            Some(target) => {
                target.source_id == pending.source_id
                    && target.relative_path == pending.relative_path
                    && target.absolute_path == pending.absolute_path
            }
            None => {
                return;
            }
        };
        if !target_matches {
            return;
        }
        let now = Instant::now();
        if let Ok(update) = message.result {
            match update {
                RecordingWaveformUpdate::NoChange { file_len } => {
                    if let Some(target) = self.audio.recording_target.as_mut() {
                        target.last_file_len = file_len;
                    }
                }
                RecordingWaveformUpdate::Updated {
                    decoded,
                    bytes,
                    file_len,
                } => {
                    if let Some(source) = self
                        .library
                        .sources
                        .iter()
                        .find(|source| source.id == pending.source_id)
                        .cloned()
                    {
                        if let Some(bytes) = bytes {
                            let _ = self.finish_waveform_load(
                                &source,
                                &pending.relative_path,
                                decoded,
                                bytes.into(),
                                AudioLoadIntent::Selection,
                                false,
                                None,
                            );
                            if let Some(target) = self.audio.recording_target.as_mut() {
                                target.loaded_once = true;
                            }
                        } else {
                            self.apply_waveform_image(decoded, None);
                        }
                    }
                    if let Some(target) = self.audio.recording_target.as_mut() {
                        target.last_file_len = file_len;
                    }
                }
            }
        }
        if let Some(target) = self.audio.recording_target.as_mut() {
            target.last_refresh_at = Some(now);
        }
    }

    fn handle_scan_message(&mut self, message: ScanJobMessage) {
        match message {
            ScanJobMessage::Progress { completed, detail } => {
                scan::handle_scan_progress(self, completed, detail);
            }
            ScanJobMessage::Finished(result) => {
                scan::handle_scan_finished(self, result);
            }
        }
    }

    fn handle_folder_scan_finished_message(&mut self, message: FolderScanResult) {
        if !self
            .runtime
            .jobs
            .folder_scan_matches(message.request_id, &message.source_id)
        {
            return;
        }
        self.runtime.jobs.clear_folder_scan();
        self.apply_folder_scan_result(message);
    }

    fn handle_trash_move_message(&mut self, message: TrashMoveMessage) {
        match message {
            TrashMoveMessage::SetTotal(total) => {
                self.ui
                    .progress
                    .set_counts(total, self.ui.progress.completed);
            }
            TrashMoveMessage::Progress { completed, detail } => {
                self.ui
                    .progress
                    .set_counts(self.ui.progress.total, completed);
                self.ui.progress.set_detail(detail);
            }
            TrashMoveMessage::Finished(result) => {
                self.runtime.jobs.clear_trash_move();
                self.apply_trash_move_finished(result);
            }
        }
    }

    fn handle_file_ops_message(&mut self, message: crate::app::controller::jobs::FileOpMessage) {
        match message {
            crate::app::controller::jobs::FileOpMessage::Progress { completed, detail } => {
                progress::update_progress_detail(
                    self,
                    ProgressTaskKind::FileOps,
                    completed,
                    detail,
                );
            }
            crate::app::controller::jobs::FileOpMessage::Finished(result) => {
                self.runtime.jobs.clear_file_ops();
                self.apply_file_op_result(result);
                if self.ui.progress.task == Some(ProgressTaskKind::FileOps) {
                    self.clear_progress();
                }
            }
        }
    }

    fn handle_analysis_failures_loaded_message(&mut self, message: AnalysisFailuresResult) {
        self.ui_cache
            .browser
            .analysis_failures_pending
            .remove(&message.source_id);
        match message.result {
            Ok(failures) => {
                if failures.is_empty() {
                    self.ui_cache
                        .browser
                        .analysis_failures
                        .remove(&message.source_id);
                } else {
                    self.ui_cache
                        .browser
                        .analysis_failures
                        .insert(message.source_id, failures);
                }
            }
            Err(err) => {
                self.ui_cache
                    .browser
                    .analysis_failures
                    .remove(&message.source_id);
                self.set_status(
                    format!("Failed to load analysis failures: {err}"),
                    StatusTone::Warning,
                );
            }
        }
    }

    fn handle_umap_built_message(&mut self, message: UmapBuildResult) {
        self.runtime.jobs.clear_umap_build();
        match message.result {
            Ok(()) => {
                self.ui.map.bounds = None;
                self.ui.map.cached_bounds_source_id = None;
                self.ui.map.cached_bounds_umap_version = None;
                self.ui.map.last_query = None;
                self.ui.map.cached_points.clear();
                self.ui.map.cached_points_source_id = None;
                self.ui.map.cached_points_umap_version = None;
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
                self.set_status(
                    format!("t-SNE layout {} built", message.umap_version),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                self.set_status(format!("t-SNE build failed: {err}"), StatusTone::Error);
            }
        }
    }

    fn handle_umap_clusters_built_message(&mut self, message: UmapClusterBuildResult) {
        self.runtime.jobs.clear_umap_cluster_build();
        match message.result {
            Ok(stats) => {
                self.ui.map.last_query = None;
                self.ui.map.cached_points.clear();
                self.ui.map.cached_points_source_id = None;
                self.ui.map.cached_points_umap_version = None;
                self.ui.map.cached_cluster_centroids_key = None;
                self.ui.map.cached_cluster_centroids = None;
                self.ui.map.auto_cluster_build_requested_key = None;
                self.mark_map_dataset_projection_revision_dirty();
                self.mark_map_query_projection_revision_dirty();
                let scope = message
                    .source_id
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("all sources");
                self.set_status(
                    format!(
                        "Clusters built for {scope} ({} clusters, {:.1}% noise)",
                        stats.cluster_count,
                        stats.noise_ratio * 100.0
                    ),
                    StatusTone::Info,
                );
            }
            Err(err) => {
                self.set_status(format!("Cluster build failed: {err}"), StatusTone::Error);
            }
        }
    }

    fn handle_source_db_maintenance_finished_message(
        &mut self,
        message: SourceDbMaintenanceResult,
    ) {
        self.runtime.jobs.clear_source_db_maintenance();
        let mut failed = 0usize;
        for outcome in message.outcomes {
            if let Some(err) = outcome.error {
                failed = failed.saturating_add(1);
                tracing::warn!(
                    "Deferred source DB maintenance failed for {} ({}): {}",
                    outcome.source_id,
                    outcome.source_root.display(),
                    err
                );
            }
        }
        if failed > 0 {
            let suffix = if failed == 1 { "" } else { "s" };
            self.set_status(
                format!("Deferred source DB maintenance failed for {failed} source{suffix}"),
                StatusTone::Warning,
            );
        }
    }

    fn handle_browser_search_finished_message(&mut self, message: SearchResult) {
        if Some(&message.source_id) == self.selection_state.ctx.selected_source.as_ref()
            && message.query == self.ui.browser.search_query
            && message.request_id == self.ui.browser.latest_search_request_id
        {
            self.ui.browser.visible = message.visible;
            self.ui.browser.visible_rows_revision =
                self.ui.browser.visible_rows_revision.wrapping_add(1);
            self.ui.browser.trash = message.trash;
            self.ui.browser.neutral = message.neutral;
            self.ui.browser.keep = message.keep;
            self.rebuild_browser_lookup_maps();
            self.ui_cache.browser.search.scores = message.scores;
            self.ui.browser.latest_applied_search_request_id = message.request_id;
            self.ui.browser.search_busy = false;

            let focused_index = self.selected_row_index();
            let loaded_index = self.loaded_row_index();
            self.ui.browser.selected_visible =
                focused_index.and_then(|idx| self.browser_visible_row_for_entry(idx));
            self.ui.browser.loaded_visible =
                loaded_index.and_then(|idx| self.browser_visible_row_for_entry(idx));
            self.ui.browser.marker_cache = None;
        }
    }

    fn handle_normalized_message(&mut self, message: NormalizationResult) {
        let source = self
            .library
            .sources
            .iter()
            .find(|s| s.id == message.source_id)
            .cloned();
        let was_playing = self.is_playing();
        let playhead_position = self.ui.waveform.playhead.position;
        let was_looping = self.ui.waveform.loop_enabled;

        match message.result {
            Ok((file_size, modified_ns, tag)) => {
                if let Some(source) = &source {
                    let updated = WavEntry {
                        relative_path: message.relative_path.clone(),
                        file_size,
                        modified_ns,
                        content_hash: None,
                        tag,
                        looped: self
                            .wav_index_for_path(&message.relative_path)
                            .and_then(|idx| self.wav_entries.entry(idx))
                            .map(|e| e.looped)
                            .unwrap_or(false),
                        missing: false,
                        last_played_at: self
                            .wav_index_for_path(&message.relative_path)
                            .and_then(|idx| self.wav_entries.entry(idx))
                            .and_then(|e| e.last_played_at),
                    };

                    let is_currently_loaded = self
                        .sample_view
                        .wav
                        .loaded_audio
                        .as_ref()
                        .is_some_and(|audio| {
                            audio.source_id == source.id
                                && audio.relative_path == message.relative_path
                        });

                    if is_currently_loaded && was_playing {
                        let start_override = if playhead_position.is_finite() {
                            Some(playhead_position.clamp(0.0, 1.0))
                        } else {
                            None
                        };
                        self.runtime
                            .jobs
                            .set_pending_playback(Some(PendingPlayback {
                                source_id: source.id.clone(),
                                relative_path: message.relative_path.clone(),
                                looped: was_looping,
                                start_override,
                            }));
                    }

                    self.update_cached_entry(source, &message.relative_path, updated);

                    if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id) {
                        self.rebuild_browser_lists();
                    }
                    self.refresh_waveform_for_sample(source, &message.relative_path);

                    self.set_status(
                        format!("Normalized {}", message.relative_path.display()),
                        StatusTone::Info,
                    );
                }
            }
            Err(err) => {
                self.set_status(format!("Normalization failed: {err}"), StatusTone::Error);
            }
        }
        if self.ui.progress.task == Some(ProgressTaskKind::Normalization) {
            self.ui.progress.completed = self.ui.progress.completed.saturating_add(1);
            if self.ui.progress.completed >= self.ui.progress.total {
                self.clear_progress();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CancelRequestAction {
    TrashMove,
    Scan,
    Analysis,
    FileOps,
    None,
}

fn cancel_request_action(task: Option<ProgressTaskKind>) -> CancelRequestAction {
    match task {
        Some(ProgressTaskKind::TrashMove) => CancelRequestAction::TrashMove,
        Some(ProgressTaskKind::Scan) => CancelRequestAction::Scan,
        Some(ProgressTaskKind::Analysis) => CancelRequestAction::Analysis,
        Some(ProgressTaskKind::FileOps) => CancelRequestAction::FileOps,
        _ => CancelRequestAction::None,
    }
}

fn pending_audio_matches(
    pending: &PendingAudio,
    request_id: u64,
    source_id: &SourceId,
    relative_path: &Path,
) -> bool {
    request_id == pending.request_id
        && source_id == &pending.source_id
        && relative_path == pending.relative_path
}

fn pending_recording_waveform_matches(
    pending: &PendingRecordingWaveform,
    request_id: u64,
    source_id: &SourceId,
    relative_path: &Path,
) -> bool {
    request_id == pending.request_id
        && source_id == &pending.source_id
        && relative_path == pending.relative_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_request_action_maps_supported_progress_tasks() {
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::TrashMove)),
            CancelRequestAction::TrashMove
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::Scan)),
            CancelRequestAction::Scan
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::Analysis)),
            CancelRequestAction::Analysis
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::FileOps)),
            CancelRequestAction::FileOps
        );
        assert_eq!(
            cancel_request_action(Some(ProgressTaskKind::WavLoad)),
            CancelRequestAction::None
        );
        assert_eq!(cancel_request_action(None), CancelRequestAction::None);
    }

    #[test]
    fn pending_audio_matches_requires_request_source_and_path_match() {
        let source = SourceId::from_string("source-a");
        let pending = PendingAudio {
            request_id: 42,
            source_id: source.clone(),
            root: std::path::PathBuf::from("/tmp/source"),
            relative_path: std::path::PathBuf::from("kick.wav"),
            intent: crate::app::controller::state::audio::AudioLoadIntent::Selection,
        };

        assert!(pending_audio_matches(
            &pending,
            42,
            &source,
            Path::new("kick.wav")
        ));
        assert!(!pending_audio_matches(
            &pending,
            41,
            &source,
            Path::new("kick.wav")
        ));
        assert!(!pending_audio_matches(
            &pending,
            42,
            &SourceId::from_string("source-b"),
            Path::new("kick.wav")
        ));
        assert!(!pending_audio_matches(
            &pending,
            42,
            &source,
            Path::new("snare.wav")
        ));
    }

    #[test]
    fn pending_recording_waveform_matches_requires_request_source_and_path_match() {
        let source = SourceId::from_string("source-a");
        let pending = PendingRecordingWaveform {
            request_id: 77,
            source_id: source.clone(),
            relative_path: std::path::PathBuf::from("recording.wav"),
            absolute_path: std::path::PathBuf::from("/tmp/source/recording.wav"),
        };

        assert!(pending_recording_waveform_matches(
            &pending,
            77,
            &source,
            Path::new("recording.wav")
        ));
        assert!(!pending_recording_waveform_matches(
            &pending,
            76,
            &source,
            Path::new("recording.wav")
        ));
        assert!(!pending_recording_waveform_matches(
            &pending,
            77,
            &SourceId::from_string("source-b"),
            Path::new("recording.wav")
        ));
        assert!(!pending_recording_waveform_matches(
            &pending,
            77,
            &source,
            Path::new("other.wav")
        ));
    }
}
