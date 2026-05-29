use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use super::{
    GuiAppState, GuiMessage, SampleLoadResult, WaveformState, emit_gui_action, sample_path_label,
};
use progress_reporter::SampleLoadProgressReporter;
pub(super) use types::{NormalizedWaveformReload, WaveformPlaybackResume};

mod cache;
mod deferred_drop;
mod progress_reporter;
mod types;

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
        self.load_sample(path, context);
    }

    pub(super) fn load_sample(
        &mut self,
        path: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
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
        let cache_lookup_started_at = Instant::now();
        if let Some(waveform) = self.cached_waveform_state(Path::new(&path)) {
            log_slow_sample_load_phase(
                "browser.select_sample.cache_lookup",
                &path,
                cache_lookup_started_at,
            );
            self.finish_cached_sample_load(waveform, autoplay, started_at);
            return;
        }
        log_slow_sample_load_phase(
            "browser.select_sample.cache_lookup",
            &path,
            cache_lookup_started_at,
        );
        if self.waveform.is_playing() {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }
        self.sample_status = format!("Loading {}", sample_path_label(path.as_str()));
        let label = sample_path_label(path.as_str());
        self.waveform_loading_label = Some(label.clone());
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&label),
            "load_queued",
            started_at,
            None,
        );
        let ticket = self.sample_load_task.begin();
        let token = ui::CancellationToken::new();
        self.sample_load_cancel = Some(token.clone());
        let sender = self.worker_sender.clone();
        context.spawn_cancellable(
            "gui-sample-load",
            token,
            move |token| {
                if token.is_cancelled() {
                    return ui::TaskCompletion {
                        ticket,
                        output: SampleLoadResult {
                            path,
                            result: Err(String::from("cancelled")),
                            autoplay,
                        },
                    };
                }
                let progress_reporter =
                    std::cell::RefCell::new(SampleLoadProgressReporter::new(sender, ticket));
                let result = WaveformState::load_path_with_progress_and_cancel(
                    PathBuf::from(&path),
                    |progress| {
                        progress_reporter.borrow_mut().report(progress);
                    },
                    || token.is_cancelled(),
                );
                ui::TaskCompletion {
                    ticket,
                    output: SampleLoadResult {
                        path,
                        result,
                        autoplay,
                    },
                }
            },
            GuiMessage::SampleLoadFinished,
        );
    }

    pub(super) fn finish_sample_load(&mut self, load: ui::TaskCompletion<SampleLoadResult>) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let load = load.output;
        let label = sample_path_label(load.path.as_str());
        if !self.sample_load_task.finish(ticket) {
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

    fn finish_cached_sample_load(
        &mut self,
        waveform: WaveformState,
        autoplay: bool,
        started_at: Instant,
    ) {
        if !autoplay && self.waveform.is_playing() {
            let stop_started_at = Instant::now();
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
            log_slow_sample_load_phase(
                "browser.select_sample.cache_finish.stop_previous",
                waveform.path().to_string_lossy().as_ref(),
                stop_started_at,
            );
        }
        let file_name = waveform.file_name();
        self.cancel_inflight_sample_load();
        self.waveform_loading_label = None;
        self.waveform_loading_progress = 0.0;
        self.waveform_loading_target_progress = 0.0;
        let replace_started_at = Instant::now();
        self.replace_waveform_deferred(waveform);
        log_slow_sample_load_phase(
            "browser.select_sample.cache_finish.replace_waveform",
            &file_name,
            replace_started_at,
        );
        if !autoplay {
            self.sample_status = format!("Loaded {file_name}");
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&file_name),
                "cache_loaded",
                started_at,
                None,
            );
            return;
        }
        let playback_started_at = Instant::now();
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                log_slow_sample_load_phase(
                    "browser.select_sample.cache_finish.start_playback",
                    &file_name,
                    playback_started_at,
                );
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "cache_playing",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                log_slow_sample_load_phase(
                    "browser.select_sample.cache_finish.start_playback",
                    &file_name,
                    playback_started_at,
                );
                self.sample_status = format!("Loaded {file_name} | playback unavailable: {err}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&file_name),
                    "cache_loaded_playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn cancel_inflight_sample_load(&mut self) {
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
