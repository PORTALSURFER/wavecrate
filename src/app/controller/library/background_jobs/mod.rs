mod analysis;
mod progress;
mod scan;
mod similarity;
mod updates;

use super::jobs::JobMessage;
use super::*;
use crate::app::controller::playback::audio_loader::AudioLoadResult;
use crate::app::controller::playback::recording::waveform_loader::RecordingWaveformUpdate;
use crate::app::controller::state::audio::AudioLoadIntent;
use crate::app::state::ProgressTaskKind;
use std::sync::atomic::Ordering;
use std::time::Instant;
use trash_move::TrashMoveMessage;

impl AppController {
    pub(crate) fn poll_background_jobs(&mut self) {
        if self.ui.progress.cancel_requested {
            match self.ui.progress.task {
                Some(ProgressTaskKind::TrashMove) => {
                    if let Some(cancel) = self.runtime.jobs.trash_move_cancel().as_ref() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                Some(ProgressTaskKind::Scan) => {
                    if let Some(cancel) = self.runtime.jobs.scan_cancel().as_ref() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                Some(ProgressTaskKind::Analysis) => {
                    self.runtime.analysis.cancel();
                    self.clear_progress();
                }
                Some(ProgressTaskKind::FileOps) => {
                    if let Some(cancel) = self.runtime.jobs.file_ops_cancel().as_ref() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                _ => {}
            }
        }

        loop {
            let message = match self.runtime.jobs.try_recv_message() {
                Ok(message) => message,
                Err(
                    std::sync::mpsc::TryRecvError::Empty
                    | std::sync::mpsc::TryRecvError::Disconnected,
                ) => {
                    break;
                }
            };

            match message {
                JobMessage::WavLoaded(message) => {
                    if Some(&message.source_id) != self.selection_state.ctx.selected_source.as_ref()
                    {
                        continue;
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
                JobMessage::AudioLoaded(message) => match message {
                    AudioLoadResult::Primary {
                        request_id,
                        source_id,
                        relative_path,
                        result,
                    } => {
                        let Some(pending) = self.runtime.jobs.pending_audio() else {
                            continue;
                        };
                        if request_id != pending.request_id
                            || source_id != pending.source_id
                            || relative_path != pending.relative_path
                        {
                            continue;
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
                },
                JobMessage::RecordingWaveformLoaded(message) => {
                    let Some(pending) = self.runtime.jobs.pending_recording_waveform() else {
                        continue;
                    };
                    if message.request_id != pending.request_id
                        || message.source_id != pending.source_id
                        || message.relative_path != pending.relative_path
                    {
                        continue;
                    }
                    self.runtime.jobs.set_pending_recording_waveform(None);
                    let target_matches = match self.audio.recording_target.as_ref() {
                        Some(target) => {
                            target.source_id == pending.source_id
                                && target.relative_path == pending.relative_path
                                && target.absolute_path == pending.absolute_path
                        }
                        None => {
                            continue;
                        }
                    };
                    if !target_matches {
                        continue;
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
                JobMessage::Scan(message) => match message {
                    ScanJobMessage::Progress { completed, detail } => {
                        scan::handle_scan_progress(self, completed, detail);
                    }
                    ScanJobMessage::Finished(result) => {
                        scan::handle_scan_finished(self, result);
                    }
                },
                JobMessage::FolderScanFinished(message) => {
                    if !self
                        .runtime
                        .jobs
                        .folder_scan_matches(message.request_id, &message.source_id)
                    {
                        continue;
                    }
                    self.runtime.jobs.clear_folder_scan();
                    self.apply_folder_scan_result(message);
                }
                JobMessage::SourceWatch(message) => {
                    self.handle_source_watch_event(&message.source_id);
                }
                JobMessage::TrashMove(message) => match message {
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
                },
                JobMessage::FolderDeleteRecoveryFinished(report) => {
                    self.apply_folder_delete_recovery_report(report);
                }
                JobMessage::FileOps(message) => match message {
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
                },
                JobMessage::Analysis(message) => {
                    analysis::handle_analysis_message(self, message);
                }
                JobMessage::AnalysisFailuresLoaded(message) => {
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
                JobMessage::UmapBuilt(message) => {
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
                            self.set_status(
                                format!("t-SNE build failed: {err}"),
                                StatusTone::Error,
                            );
                        }
                    }
                }
                JobMessage::UmapClustersBuilt(message) => {
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
                            self.set_status(
                                format!("Cluster build failed: {err}"),
                                StatusTone::Error,
                            );
                        }
                    }
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
                            format!(
                                "Deferred source DB maintenance failed for {failed} source{suffix}"
                            ),
                            StatusTone::Warning,
                        );
                    }
                }
                JobMessage::BrowserSearchFinished(message) => {
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

                        // Re-sync selection/loaded hints for the new visible list
                        let focused_index = self.selected_row_index();
                        let loaded_index = self.loaded_row_index();
                        self.ui.browser.selected_visible =
                            focused_index.and_then(|idx| self.browser_visible_row_for_entry(idx));
                        self.ui.browser.loaded_visible =
                            loaded_index.and_then(|idx| self.browser_visible_row_for_entry(idx));
                        self.ui.browser.marker_cache = None;
                    }
                }
                JobMessage::Normalized(message) => {
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

                                let is_currently_loaded =
                                    self.sample_view.wav.loaded_audio.as_ref().is_some_and(
                                        |audio| {
                                            audio.source_id == source.id
                                                && audio.relative_path == message.relative_path
                                        },
                                    );

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

                                if self.selection_state.ctx.selected_source.as_ref()
                                    == Some(&source.id)
                                {
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
                            self.set_status(
                                format!("Normalization failed: {err}"),
                                StatusTone::Error,
                            );
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
        }
    }
}
