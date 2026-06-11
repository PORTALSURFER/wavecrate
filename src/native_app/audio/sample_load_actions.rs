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
    GuiMessage, NativeAppState, WaveformState, emit_gui_action, sample_path_label,
};
use crate::native_app::waveform::cached_waveform_file_playback_ready_exists;
use types::SampleLoadStrategy;
pub(in crate::native_app) use types::{NormalizedWaveformReload, WaveformPlaybackResume};

mod cache;
mod completion;
mod deferred_drop;
mod plan;
mod types;
mod worker;

#[cfg(test)]
pub(in crate::native_app) use cache::{
    active_folder_cache_warm_priority, warm_active_folder_waveform_cache,
    warm_persisted_waveform_cache,
};

pub(in crate::native_app) fn foreground_sample_load_priority() -> ui::TaskPriority {
    ui::TaskPriority::Interactive
}

impl NativeAppState {
    pub(in crate::native_app) fn reload_normalized_waveform(
        &mut self,
        reload: NormalizedWaveformReload<'_>,
    ) -> Result<(), String> {
        self.replace_waveform_deferred(WaveformState::load_path(reload.path.to_path_buf())?);
        self.library
            .folder_browser
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
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        self.library
            .folder_browser
            .focus_file_preserving_selection(path.clone());
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
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
        let previous_selection = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        self.library
            .folder_browser
            .select_file_with_modifiers(path.clone(), modifiers);
        if self.library.folder_browser.selected_file_id() != previous_selection.as_deref() {
            self.cancel_metadata_tag_entry();
            self.metadata.selected_tag = None;
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
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        if self.start_loaded_navigation_sample(path.as_str(), context, started_at) {
            return;
        }
        if self.start_memory_cached_sample(path.as_str(), true, context, started_at) {
            return;
        }
        self.ui.status.sample = format!("Selected {}", sample_path_label(path.as_str()));
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
            true,
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
            .waveform
            .cache
            .entries
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
            self.ui.status.sample = format!("Loaded {file_name}");
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
                self.ui.status.sample = format!("Playing {file_name}");
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
                self.ui.status.sample = format!("Playing {file_name} when audio output is ready");
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
                self.ui.status.sample = format!("Loaded {file_name} | playback unavailable: {err}");
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
            SampleLoadStrategy::PersistedPlaybackCacheOnly,
        );
        true
    }

    pub(super) fn clear_sample_loading_state(&mut self) {
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        self.background.sample_load_cancel = None;
    }

    pub(in crate::native_app) fn waveform_sample_load_active(&self) -> bool {
        self.background.deferred_sample_load_task.active().is_some()
            || self.background.sample_load_task.active().is_some()
    }

    pub(in crate::native_app) fn waveform_input_blocked_by_sample_load(&self) -> bool {
        self.waveform.load.label.is_some()
            && self.waveform_sample_load_active()
            && !self.library.folder_browser.drag_active()
    }

    fn stop_current_sample_playback_for_load(&mut self) {
        if !self.waveform.current.is_playing() && self.audio.early_sample_playback_path.is_none() {
            return;
        }
        if let Some(player) = self.audio.player.as_mut() {
            player.stop();
        }
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.early_sample_playback_path = None;
    }

    fn start_loaded_navigation_sample(
        &mut self,
        path: &str,
        context: &mut ui::UpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        if !self.waveform.current.has_loaded_sample()
            || self.waveform.current.path() != Path::new(path)
        {
            return false;
        }

        self.maybe_open_audio_player(context);
        let file_name = self.waveform.current.file_name();
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                self.ui.status.sample = format!("Playing {file_name}");
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
                self.ui.status.sample = format!("Playing {file_name} when audio output is ready");
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
                self.ui.status.sample = format!("Loaded {file_name} | playback unavailable: {err}");
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

pub(super) fn log_slow_sample_load_phase(event: &'static str, source: &str, started_at: Instant) {
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
