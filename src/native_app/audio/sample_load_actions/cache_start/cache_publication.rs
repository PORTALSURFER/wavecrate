use super::*;

impl NativeAppState {
    pub(in crate::native_app) fn finish_preview_audition_decode(
        &mut self,
        completion: ui::TaskCompletion<PreviewAuditionResult>,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(result) = self
            .background
            .preview_audition_task
            .finish_completion(completion)
        else {
            return;
        };
        if !self.preview_audition_decode_matches_current_target(result.path.as_str()) {
            log_sample_load_timing(
                "browser.sample_load.preview_audition.decode_stale",
                result.path.as_str(),
                started_at.elapsed(),
                false,
            );
            self.record_preview_audition_decode_stale(result.path.as_str(), started_at);
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(result.path.as_str())),
                "preview_audition_stale",
                started_at,
                None,
            );
            return;
        }
        let clip = match result.clip {
            Ok(clip) => clip,
            Err(error) => {
                log_sample_load_timing(
                    "browser.sample_load.preview_audition.decode_error",
                    result.path.as_str(),
                    started_at.elapsed(),
                    false,
                );
                self.waveform
                    .cache
                    .mark_preview_audition_failed(Path::new(result.path.as_str()));
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&sample_path_label(result.path.as_str())),
                    "preview_audition_error",
                    started_at,
                    Some(&error),
                );
                self.advance_starmap_audition_after_preview_decode_failure(
                    result.path.as_str(),
                    context,
                );
                return;
            }
        };
        self.waveform
            .cache
            .store_preview_audition_clip(clip.clone());
        if self.waveform.current.has_loaded_sample()
            && self.waveform.current.path() == Path::new(result.path.as_str())
        {
            log_sample_load_timing(
                "browser.sample_load.preview_audition.decode_superseded_by_full_load",
                result.path.as_str(),
                started_at.elapsed(),
                false,
            );
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(result.path.as_str())),
                "preview_audition_superseded_by_full_load",
                started_at,
                None,
            );
            return;
        }
        log_sample_load_timing(
            "browser.sample_load.preview_audition.decode_ready",
            result.path.as_str(),
            started_at.elapsed(),
            false,
        );
        let options = FastAuditionOptions::preview_decode_completion(
            self.runtime_playback_origin_for_path(result.path.as_str()),
            !self.ui.chrome.starmap_audition_drag.is_some(),
        );
        let outcome =
            self.start_fast_path_audition(result.path.as_str(), context, started_at, options);
        if outcome == InstantAuditionOutcome::Started
            && self.ui.chrome.starmap_audition_drag.is_some()
            && !self
                .ui
                .chrome
                .starmap_audition_queue
                .queued_file_ids
                .is_empty()
        {
            self.advance_starmap_drag_audition_latest_immediately(context);
        }
    }

    pub(super) fn preview_audition_decode_matches_current_target(&self, path: &str) -> bool {
        if self.ui.chrome.starmap_audition_drag.is_some()
            || self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some()
        {
            return self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .as_deref()
                == Some(path);
        }
        self.library.folder_browser.selected_file_id() == Some(path)
    }

    fn advance_starmap_audition_after_preview_decode_failure(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref()
            != Some(path)
        {
            return;
        }
        self.ui.chrome.starmap_audition_queue.active_file_id = None;
        context.request_paint_only();
        self.start_next_starmap_audition_hit(context);
    }

    fn record_preview_audition_decode_stale(&self, path: &str, started_at: Instant) {
        let starmap_active = self.ui.chrome.starmap_audition_drag.is_some()
            || self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some()
            || !self
                .ui
                .chrome
                .starmap_audition_queue
                .queued_file_ids
                .is_empty();
        starmap_telemetry::record_event(
            None,
            "preview_decode.finish",
            if starmap_active {
                "stale_starmap_target"
            } else {
                "stale_selection"
            },
            Some(path),
            0,
            self.ui.chrome.starmap_audition_queue.queued_file_ids.len(),
            self.ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some(),
            Some(started_at.elapsed()),
        );
    }

    pub(in crate::native_app) fn finish_preview_audition_warm(
        &mut self,
        completion: ui::TaskCompletion<PreviewAuditionWarmResult>,
        started_at: Instant,
    ) {
        let finish_started_at = Instant::now();
        let Some(result) = self
            .background
            .preview_audition_warm_task
            .finish_completion(completion)
        else {
            return;
        };
        self.waveform.cache.finish_preview_audition_warm_schedule(
            &result.scheduled_paths,
            &result.attempted_paths,
            &result.failed_paths,
        );
        let scheduled_count = result.scheduled_paths.len();
        let attempted_count = result.attempted_paths.len();
        let clip_count = result.clips.len();
        let waveform_preview_count = result.waveform_previews.len();
        let error_count = result.errors;
        for clip in result.clips {
            self.waveform.cache.store_preview_audition_clip(clip);
        }
        for preview in result.waveform_previews {
            self.waveform.cache.store_instant_waveform_preview(preview);
        }
        let worker_elapsed = started_at.elapsed();
        let commit_elapsed = finish_started_at.elapsed();
        record_preview_audition_warm_finished(
            scheduled_count,
            attempted_count,
            clip_count,
            error_count,
            worker_elapsed,
            commit_elapsed,
        );
        log_sample_load_timing(
            "browser.sample_load.preview_audition.warm_commit",
            "preview-audition-warm",
            commit_elapsed,
            false,
        );
        tracing::debug!(
            target: "wavecrate::debug::sample_load",
            event = "browser.sample_load.preview_audition.warm_finished",
            scheduled = scheduled_count,
            attempted = attempted_count,
            decoded = clip_count,
            waveform_previews = waveform_preview_count,
            errors = error_count,
            worker_elapsed_ms = worker_elapsed.as_secs_f64() * 1000.0,
            commit_elapsed_ms = commit_elapsed.as_secs_f64() * 1000.0,
            "Preview audition warm finished"
        );
        if error_count > 0 {
            tracing::debug!(
                attempted = attempted_count,
                decoded = clip_count,
                errors = error_count,
                "Preview audition warm finished with decode misses"
            );
        }
    }
}
