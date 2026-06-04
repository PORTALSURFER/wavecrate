use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use super::{
    GuiAppState, GuiMessage, KEYBOARD_SAMPLE_LOAD_DEBOUNCE, PendingSamplePlayback,
    SampleLoadResult, UNCACHED_SAMPLE_LOAD_DEBOUNCE, WaveformState, emit_gui_action,
    sample_path_label,
};
pub(super) use types::{NormalizedWaveformReload, WaveformPlaybackResume};

mod cache;
mod deferred_drop;
mod types;

#[cfg(test)]
pub(in crate::gui_app) use cache::warm_persisted_waveform_cache;

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;

impl GuiAppState {
    pub(super) fn reload_normalized_waveform(
        &mut self,
        reload: NormalizedWaveformReload<'_>,
    ) -> Result<(), String> {
        self.replace_waveform_deferred(WaveformState::load_path(reload.path.to_path_buf())?);
        self.folder_browser
            .select_file(reload.path.display().to_string());
        if let Some(playback) = reload.playback {
            let (_, previous_end) = playback.span.unwrap_or((0.0, 1.0));
            let start = playback.start_ratio.clamp(0.0, 1.0);
            let end = previous_end.max(start).clamp(start, 1.0);
            self.start_playback_current_span(start, end)?;
        }
        Ok(())
    }

    pub(super) fn select_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let previous_selection = self.folder_browser.selected_file_id().map(str::to_owned);
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        if self.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.selected_metadata_tag = None;
        }
        self.pending_sample_playback = None;
        self.load_sample(path, context);
    }

    pub(super) fn select_sample_with_modifiers(
        &mut self,
        path: String,
        modifiers: PointerModifiers,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let previous_selection = self.folder_browser.selected_file_id().map(str::to_owned);
        self.folder_browser
            .select_file_with_modifiers(path.clone(), modifiers);
        if self.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.selected_metadata_tag = None;
        }
        self.pending_sample_playback = None;
        self.load_sample(path, context);
    }

    pub(super) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.pending_sample_playback = None;
        self.load_sample_with_autoplay(path, context, true);
    }

    pub(super) fn load_sample_without_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.load_sample_with_autoplay(path, context, false);
    }

    fn load_sample_with_autoplay(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
        autoplay: bool,
    ) {
        let started_at = Instant::now();
        self.cancel_inflight_sample_load();
        self.prepare_uncached_sample_load(path.as_str(), "load_deferred", started_at);
        self.schedule_deferred_sample_load(
            path,
            autoplay,
            false,
            UNCACHED_SAMPLE_LOAD_DEBOUNCE,
            context,
        );
    }

    pub(super) fn defer_navigation_sample_load(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.cancel_inflight_sample_load();
        self.pending_sample_playback = None;
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        if self.start_loaded_navigation_sample(path.as_str(), context, started_at) {
            return;
        }
        self.sample_status = format!("Selected {}", sample_path_label(path.as_str()));
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path.as_str())),
            "navigation_load_deferred",
            started_at,
            None,
        );
        self.schedule_deferred_sample_load(
            path,
            true,
            false,
            KEYBOARD_SAMPLE_LOAD_DEBOUNCE,
            context,
        );
    }

    fn start_loaded_navigation_sample(
        &mut self,
        path: &str,
        context: &mut ui::UpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        if !self.waveform.has_loaded_sample() || self.waveform.path() != Path::new(path) {
            return false;
        }

        self.maybe_open_audio_player(context);
        let file_name = self.waveform.file_name();
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "loaded_playback_started",
                    started_at,
                    None,
                );
            }
            Err(err) if self.pending_playback_start.is_some() => {
                self.sample_status = format!("Playing {file_name} when audio output is ready");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "loaded_playback_pending",
                    started_at,
                    Some(&err),
                );
            }
            Err(err) => {
                self.sample_status = format!("Loaded {file_name} | playback unavailable: {err}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "loaded_playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
        true
    }

    fn schedule_deferred_sample_load(
        &mut self,
        path: String,
        autoplay: bool,
        check_cache: bool,
        delay: Duration,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        context.after_latest(&mut self.deferred_sample_load_task, delay, |ticket| {
            GuiMessage::DeferredSampleLoad {
                ticket,
                path,
                autoplay,
                check_cache,
            }
        });
    }

    pub(super) fn start_deferred_sample_load(
        &mut self,
        ticket: ui::TaskTicket,
        path: String,
        autoplay: bool,
        _check_cache: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.deferred_sample_load_task.finish(ticket)
            || self.folder_browser.selected_file_id() != Some(path.as_str())
        {
            self.pending_sample_playback = None;
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path.as_str())),
                "load_deferred_stale",
                started_at,
                None,
            );
            return;
        }
        self.start_uncached_sample_load(path, autoplay, context, started_at);
    }

    fn prepare_uncached_sample_load(
        &mut self,
        path: &str,
        outcome: &'static str,
        started_at: Instant,
    ) {
        if self.waveform.is_playing() {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }
        self.sample_status = format!("Loading {}", sample_path_label(path));
        let label = sample_path_label(path);
        self.waveform_loading_label = Some(label.clone());
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            outcome,
            started_at,
            None,
        );
    }

    fn start_uncached_sample_load(
        &mut self,
        path: String,
        autoplay: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path.as_str())),
            "load_queued",
            started_at,
            None,
        );
        let sender = self.worker_sender.clone();
        self.sample_load_cancel = Some(context.spawn_cancellable_latest_with_priority(
            &mut self.sample_load_task,
            "gui-sample-load",
            ui::TaskPriority::Idle,
            move |ticket, token| {
                if token.is_cancelled() {
                    return SampleLoadResult {
                        path,
                        result: Err(String::from("cancelled")),
                        autoplay,
                    };
                }
                let progress_gate = ui::ProgressUpdateGate::new(
                    SAMPLE_LOAD_PROGRESS_MIN_INTERVAL,
                    SAMPLE_LOAD_PROGRESS_MIN_DELTA,
                )
                .with_max_fraction(0.995);
                let progress_reporter = std::cell::RefCell::new(
                    ui::ThrottledProgressReporter::new(progress_gate, move |progress| {
                        let _ = sender.send(GuiMessage::SampleLoadProgress(ticket, progress));
                    }),
                );
                let result = WaveformState::load_path_with_progress_and_cancel(
                    PathBuf::from(&path),
                    |progress| {
                        progress_reporter.borrow_mut().report(progress);
                    },
                    || token.is_cancelled(),
                );
                SampleLoadResult {
                    path,
                    result,
                    autoplay,
                }
            },
            GuiMessage::SampleLoadFinished,
        ));
    }

    pub(super) fn finish_sample_load(&mut self, load: ui::TaskCompletion<SampleLoadResult>) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let load = load.output;
        let label = sample_path_label(load.path.as_str());
        if !self.sample_load_task.finish(ticket) {
            self.pending_sample_playback = None;
            emit_gui_action(
                "browser.sample_load.finish",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        self.sample_load_cancel = None;
        match load.result {
            Ok(waveform) => {
                let file_name = waveform.file_name();
                let remember_started_at = Instant::now();
                self.remember_waveform(&waveform);
                log_slow_sample_load_phase(
                    "browser.sample_load.finish.remember_cache",
                    &file_name,
                    remember_started_at,
                );
                let replace_started_at = Instant::now();
                self.replace_waveform_deferred(waveform);
                log_slow_sample_load_phase(
                    "browser.sample_load.finish.replace_waveform",
                    &file_name,
                    replace_started_at,
                );
                if self.start_pending_sample_playback(&file_name, started_at) {
                    return;
                }
                if !load.autoplay {
                    self.sample_status = format!("Loaded {file_name}");
                    emit_gui_action(
                        "browser.sample_load.finish",
                        Some("browser"),
                        Some(&file_name),
                        "loaded",
                        started_at,
                        None,
                    );
                    return;
                }
                let playback_started_at = Instant::now();
                match self.start_playback_current_span(0.0, 1.0) {
                    Ok(()) => {
                        log_slow_sample_load_phase(
                            "browser.sample_load.finish.start_playback",
                            &file_name,
                            playback_started_at,
                        );
                        self.sample_status = format!("Playing {file_name}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "playing",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        log_slow_sample_load_phase(
                            "browser.sample_load.finish.start_playback",
                            &file_name,
                            playback_started_at,
                        );
                        self.sample_status =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                        emit_gui_action(
                            "browser.sample_load.finish",
                            Some("browser"),
                            Some(&file_name),
                            "loaded_playback_error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
            }
            Err(err) => {
                self.pending_sample_playback = None;
                self.sample_status = format!("Could not load sample: {err}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn start_pending_sample_playback(&mut self, file_name: &str, started_at: Instant) -> bool {
        let Some(playback) = self.pending_sample_playback.take() else {
            return false;
        };
        match playback {
            PendingSamplePlayback::RandomAudition { unit } => {
                let span = self.random_audition_span_for_loaded_waveform(unit);
                let was_looping = self.loop_playback;
                self.loop_playback = false;
                match self.start_playback_current_span(span.start, span.end) {
                    Ok(()) => {
                        self.sample_status = span.status_message(file_name);
                        emit_gui_action(
                            "playback.play_random_sample_range",
                            Some("transport"),
                            Some(file_name),
                            "success",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        if self.pending_playback_start.is_none() {
                            self.loop_playback = was_looping;
                        }
                        self.sample_status = format!("Playback unavailable: {err}");
                        emit_gui_action(
                            "playback.play_random_sample_range",
                            Some("transport"),
                            Some(file_name),
                            "error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
                true
            }
        }
    }

    fn cancel_inflight_sample_load(&mut self) {
        self.deferred_sample_load_task.cancel();
        if let Some(token) = self.sample_load_cancel.take() {
            token.cancel();
        }
        self.sample_load_task.cancel();
    }
}

fn log_slow_sample_load_phase(event: &'static str, source: &str, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(4) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_load",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        source,
        "Slow sample load UI phase"
    );
}
