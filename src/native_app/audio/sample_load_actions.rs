use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub(in crate::native_app) const UNCACHED_SAMPLE_LOAD_DEBOUNCE: Duration = Duration::from_millis(90);
pub(in crate::native_app) const KEYBOARD_SAMPLE_LOAD_DEBOUNCE: Duration =
    UNCACHED_SAMPLE_LOAD_DEBOUNCE;

use crate::native_app::app::{
    GuiMessage, NativeAppState, PendingSamplePlayback, SampleLoadResult, SamplePlaybackReady,
    WaveformState, emit_gui_action, sample_path_label,
};
use crate::native_app::waveform::cached_waveform_file_playback_ready_exists;
pub(in crate::native_app) use types::{NormalizedWaveformReload, WaveformPlaybackResume};

mod cache;
mod deferred_drop;
mod types;

#[cfg(test)]
pub(in crate::native_app) use cache::{
    active_folder_cache_warm_priority, warm_active_folder_waveform_cache,
    warm_persisted_waveform_cache,
};

const SAMPLE_LOAD_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);
const SAMPLE_LOAD_PROGRESS_MIN_DELTA: f32 = 0.01;

pub(in crate::native_app) fn foreground_sample_load_priority() -> ui::TaskPriority {
    ui::TaskPriority::Interactive
}

impl NativeAppState {
    pub(in crate::native_app) fn reload_normalized_waveform(
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

    pub(in crate::native_app) fn select_sample(
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
        self.audio.pending_sample_playback = None;
        self.load_sample(path, context);
    }

    pub(in crate::native_app) fn select_sample_with_modifiers(
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
        self.audio.pending_sample_playback = None;
        self.load_sample(path, context);
    }

    pub(in crate::native_app) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.audio.pending_sample_playback = None;
        self.load_sample_with_autoplay(path, context, true);
    }

    pub(in crate::native_app) fn load_sample_without_autoplay(
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
        if self.start_memory_cached_sample(path.as_str(), autoplay, context, started_at) {
            return;
        }
        if self.start_persisted_cached_sample_load(path.as_str(), autoplay, context, started_at) {
            return;
        }
        self.prepare_uncached_sample_load(path.as_str(), "load_deferred", started_at);
        self.schedule_deferred_sample_load(
            path,
            autoplay,
            false,
            UNCACHED_SAMPLE_LOAD_DEBOUNCE,
            "mouse_or_direct",
            context,
        );
    }

    pub(in crate::native_app) fn defer_navigation_sample_load(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.cancel_inflight_sample_load();
        self.audio.pending_sample_playback = None;
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        if self.start_loaded_navigation_sample(path.as_str(), context, started_at) {
            return;
        }
        if self.start_memory_cached_sample(path.as_str(), true, context, started_at) {
            return;
        }
        if self.start_persisted_cached_sample_load(path.as_str(), true, context, started_at) {
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
            "keyboard",
            context,
        );
    }

    fn start_memory_cached_sample(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        let Some(file) = self
            .waveform_cache
            .get(Path::new(path))
            .map(|entry| std::sync::Arc::clone(&entry.file))
        else {
            return false;
        };
        let waveform = WaveformState::from_cached_file(file);
        let file_name = waveform.file_name();
        self.touch_cached_waveform_path(PathBuf::from(path));
        self.stop_current_sample_playback_for_load();
        self.clear_sample_loading_state();
        self.replace_waveform_deferred(waveform);
        if !autoplay {
            self.sample_status = format!("Loaded {file_name}");
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&file_name),
                "memory_cache_loaded",
                started_at,
                None,
            );
            return true;
        }
        self.maybe_open_audio_player(context);
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "memory_cache_playing",
                    started_at,
                    None,
                );
            }
            Err(err) if self.audio.pending_playback_start.is_some() => {
                self.sample_status = format!("Playing {file_name} when audio output is ready");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "memory_cache_pending",
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
                    "memory_cache_playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
        true
    }

    fn start_persisted_cached_sample_load(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        if !cached_waveform_file_playback_ready_exists(Path::new(path)) {
            return false;
        }
        self.prepare_uncached_sample_load(path, "persistent_cache_load_queued", started_at);
        self.start_sample_load_with_priority(
            path.to_owned(),
            autoplay,
            context,
            ui::TaskPriority::Interactive,
            true,
        );
        true
    }

    fn clear_sample_loading_state(&mut self) {
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        self.background.sample_load_cancel = None;
    }

    pub(in crate::native_app) fn waveform_sample_load_active(&self) -> bool {
        self.background.deferred_sample_load_task.active().is_some()
            || self.background.sample_load_task.active().is_some()
    }

    pub(in crate::native_app) fn waveform_input_blocked_by_sample_load(&self) -> bool {
        self.waveform_loading_label.is_some()
            && self.waveform_sample_load_active()
            && !self.folder_browser.drag_active()
    }

    fn stop_current_sample_playback_for_load(&mut self) {
        if !self.waveform.is_playing() && self.audio.early_sample_playback_path.is_none() {
            return;
        }
        if let Some(player) = self.audio.player.as_mut() {
            player.stop();
        }
        self.waveform.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.early_sample_playback_path = None;
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
            Err(err) if self.audio.pending_playback_start.is_some() => {
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
        input_method: &'static str,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        tracing::info!(
            target: "wavecrate::debug::sample_load",
            event = "browser.sample_load.deferred_scheduled",
            path = %path,
            input_method,
            cache_state = "uncached",
            autoplay,
            delay_ms = delay.as_secs_f64() * 1000.0,
            "Sample load scheduled"
        );
        context.after_latest(
            &mut self.background.deferred_sample_load_task,
            delay,
            |ticket| GuiMessage::DeferredSampleLoad {
                ticket,
                path,
                autoplay,
                check_cache,
                scheduled_at: Instant::now(),
            },
        );
    }

    pub(in crate::native_app) fn start_deferred_sample_load(
        &mut self,
        ticket: ui::TaskTicket,
        path: String,
        autoplay: bool,
        _check_cache: bool,
        scheduled_at: Instant,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        log_sample_load_timing(
            "browser.sample_load.deferred_wait",
            path.as_str(),
            started_at.saturating_duration_since(scheduled_at),
            true,
        );
        if !self.background.deferred_sample_load_task.finish(ticket)
            || self.folder_browser.selected_file_id() != Some(path.as_str())
        {
            self.audio.pending_sample_playback = None;
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
        self.stop_current_sample_playback_for_load();
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
        self.start_sample_load_with_priority(
            path,
            autoplay,
            context,
            foreground_sample_load_priority(),
            false,
        );
    }

    fn start_sample_load_with_priority(
        &mut self,
        path: String,
        autoplay: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
        priority: ui::TaskPriority,
        persisted_cache_only: bool,
    ) {
        let sender = self.background.worker_sender.clone();
        let queued_at = Instant::now();
        let source = path.clone();
        self.background.sample_load_cancel = Some(context.spawn_cancellable_latest_with_priority(
            &mut self.background.sample_load_task,
            "gui-sample-load",
            priority,
            move |ticket, token| {
                log_sample_load_timing(
                    "browser.sample_load.worker.queue_wait",
                    source.as_str(),
                    queued_at.elapsed(),
                    true,
                );
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
                let progress_sender = sender.clone();
                let progress_reporter = std::cell::RefCell::new(
                    ui::ThrottledProgressReporter::new(progress_gate, move |progress| {
                        let _ =
                            progress_sender.send(GuiMessage::SampleLoadProgress(ticket, progress));
                    }),
                );
                let result = if persisted_cache_only {
                    let phase_started_at = Instant::now();
                    let result = WaveformState::load_persisted_playback_cache(PathBuf::from(&path));
                    log_sample_load_timing(
                        "browser.sample_load.worker.persisted_cache",
                        path.as_str(),
                        phase_started_at.elapsed(),
                        true,
                    );
                    log_loaded_sample_metadata(path.as_str(), &result, "persisted_playback_cache");
                    result
                } else {
                    let phase_started_at = Instant::now();
                    let ready_sender = sender.clone();
                    let ready_path = path.clone();
                    let result = WaveformState::load_path_with_progress_cancel_and_playback_ready(
                        PathBuf::from(&path),
                        |progress| {
                            progress_reporter.borrow_mut().report(progress);
                        },
                        || token.is_cancelled(),
                        |audio| {
                            if autoplay && !token.is_cancelled() {
                                let _ = ready_sender.send(GuiMessage::SamplePlaybackReady(
                                    ui::TaskCompletion {
                                        ticket,
                                        output: SamplePlaybackReady {
                                            path: ready_path.clone(),
                                            audio,
                                            autoplay,
                                        },
                                    },
                                ));
                            }
                        },
                    );
                    log_sample_load_timing(
                        "browser.sample_load.worker.decode_waveform",
                        path.as_str(),
                        phase_started_at.elapsed(),
                        true,
                    );
                    log_loaded_sample_metadata(path.as_str(), &result, "uncached_decode");
                    result
                };
                SampleLoadResult {
                    path,
                    result,
                    autoplay,
                }
            },
            GuiMessage::SampleLoadFinished,
        ));
    }

    pub(in crate::native_app) fn finish_sample_load(
        &mut self,
        load: ui::TaskCompletion<SampleLoadResult>,
    ) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let load = load.output;
        let label = sample_path_label(load.path.as_str());
        if !self.background.sample_load_task.finish(ticket) {
            self.audio.pending_sample_playback = None;
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
        self.clear_sample_loading_state();
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
                if self.continue_early_sample_playback(&load.path, &file_name, started_at) {
                    return;
                }
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
                self.audio.pending_sample_playback = None;
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

    pub(in crate::native_app) fn finish_sample_playback_ready(
        &mut self,
        ready: ui::TaskCompletion<SamplePlaybackReady>,
    ) {
        let started_at = Instant::now();
        let ticket = ready.ticket;
        let ready = ready.output;
        let label = sample_path_label(ready.path.as_str());
        if !self.background.sample_load_task.is_active(ticket)
            || self.folder_browser.selected_file_id() != Some(ready.path.as_str())
        {
            emit_gui_action(
                "browser.sample_load.playback_ready",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        if !ready.autoplay {
            return;
        }
        let Some(player) = self.audio.player.as_mut() else {
            emit_gui_action(
                "browser.sample_load.playback_ready",
                Some("browser"),
                Some(&label),
                "audio_output_pending",
                started_at,
                None,
            );
            return;
        };
        let duration = ready.audio.frames as f32 / ready.audio.sample_rate.max(1) as f32;
        let output_setup_started_at = Instant::now();
        player.set_volume(self.audio.volume);
        self.audio.output_resolved = Some(player.output_details().clone());
        log_slow_sample_load_phase(
            "browser.sample_load.playback_ready.output_setup",
            &label,
            output_setup_started_at,
        );
        let set_audio_started_at = Instant::now();
        player.set_audio_samples_with_metadata(
            ready.audio.audio_bytes,
            ready.audio.playback_samples,
            duration,
            ready.audio.sample_rate,
            ready.audio.channels,
        );
        log_slow_sample_load_phase(
            "browser.sample_load.playback_ready.set_audio",
            &label,
            set_audio_started_at,
        );
        let play_started_at = Instant::now();
        match player.play_range(0.0, 1.0, false) {
            Ok(()) => {
                self.audio.early_sample_playback_path = Some(ready.path);
                self.audio.current_playback_span = Some((0.0, 1.0));
                self.sample_status = format!("Playing {label}");
                log_slow_sample_load_phase(
                    "browser.sample_load.playback_ready.player_play",
                    &label,
                    play_started_at,
                );
                emit_gui_action(
                    "browser.sample_load.playback_ready",
                    Some("browser"),
                    Some(&label),
                    "playing",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.audio.early_sample_playback_path = None;
                self.audio.current_playback_span = None;
                self.sample_status = format!("Loaded {label} | playback unavailable: {err}");
                emit_gui_action(
                    "browser.sample_load.playback_ready",
                    Some("browser"),
                    Some(&label),
                    "playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn continue_early_sample_playback(
        &mut self,
        path: &str,
        file_name: &str,
        started_at: Instant,
    ) -> bool {
        if self.audio.early_sample_playback_path.as_deref() != Some(path) {
            return false;
        }
        let progress = self
            .audio
            .player
            .as_ref()
            .and_then(|player| player.progress())
            .unwrap_or(0.0);
        self.waveform.start_playback(progress);
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.early_sample_playback_path = None;
        self.sample_status = format!("Playing {file_name}");
        emit_gui_action(
            "browser.sample_load.finish",
            Some("browser"),
            Some(file_name),
            "waveform_ready_playback_continued",
            started_at,
            None,
        );
        true
    }

    fn start_pending_sample_playback(&mut self, file_name: &str, started_at: Instant) -> bool {
        let Some(playback) = self.audio.pending_sample_playback.take() else {
            return false;
        };
        match playback {
            PendingSamplePlayback::RandomAudition { unit } => {
                let span = self.random_audition_span_for_loaded_waveform(unit);
                let was_looping = self.audio.loop_playback;
                self.audio.loop_playback = false;
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
                        if self.audio.pending_playback_start.is_none() {
                            self.audio.loop_playback = was_looping;
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
        self.background.deferred_sample_load_task.cancel();
        if let Some(token) = self.background.sample_load_cancel.take() {
            token.cancel();
        }
        self.background.sample_load_task.cancel();
        if self.audio.early_sample_playback_path.is_some() {
            if let Some(player) = self.audio.player.as_mut() {
                player.stop();
            }
            self.audio.current_playback_span = None;
        }
        self.audio.early_sample_playback_path = None;
    }
}

fn log_slow_sample_load_phase(event: &'static str, source: &str, started_at: Instant) {
    let elapsed = started_at.elapsed();
    log_sample_load_timing(event, source, elapsed, false);
}

fn log_sample_load_timing(event: &'static str, source: &str, elapsed: Duration, always: bool) {
    if !always && elapsed < Duration::from_millis(4) {
        return;
    }
    if always {
        tracing::info!(
            target: "wavecrate::debug::sample_load",
            event,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            source,
            "Sample load timing"
        );
    } else {
        tracing::warn!(
            target: "wavecrate::debug::sample_load",
            event,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            source,
            "Slow sample load UI phase"
        );
    }
}

fn log_loaded_sample_metadata(
    source: &str,
    result: &Result<WaveformState, String>,
    cache_state: &'static str,
) {
    let Ok(waveform) = result else {
        return;
    };
    tracing::info!(
        target: "wavecrate::debug::sample_load",
        event = "browser.sample_load.worker.loaded_metadata",
        source,
        cache_state,
        sample_rate = waveform.sample_rate(),
        channels = waveform.channels(),
        frames = waveform.frames(),
        file_size_bytes = waveform.audio_bytes().len(),
        playback_ready = waveform.playback_samples().is_some() || waveform.playback_cache_file().is_some(),
        "Loaded sample metadata"
    );
}
