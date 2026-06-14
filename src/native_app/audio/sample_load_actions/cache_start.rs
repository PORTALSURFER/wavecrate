use radiant::prelude as ui;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, WaveformState, emit_gui_action},
    audio::sample_load_actions::types::SampleLoadStrategy,
};

struct CachedPlaybackOutcomes {
    playing: &'static str,
    pending: &'static str,
    error: &'static str,
}

const MEMORY_CACHE_OUTCOMES: CachedPlaybackOutcomes = CachedPlaybackOutcomes {
    playing: "memory_cache_playing",
    pending: "memory_cache_pending",
    error: "memory_cache_playback_error",
};

const LOADED_NAVIGATION_OUTCOMES: CachedPlaybackOutcomes = CachedPlaybackOutcomes {
    playing: "loaded_playback_started",
    pending: "loaded_playback_pending",
    error: "loaded_playback_error",
};

impl NativeAppState {
    pub(super) fn start_memory_cached_sample(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
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
        self.start_cached_sample_playback(&file_name, MEMORY_CACHE_OUTCOMES, started_at);
        true
    }

    pub(super) fn start_foreground_sample_load(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        self.prepare_uncached_sample_load(path, "preferred_cache_load_queued", started_at);
        self.start_sample_load_with_priority(
            path.to_owned(),
            autoplay,
            context,
            ui::TaskPriority::Interactive,
            SampleLoadStrategy::PreferPersistedPlaybackCache,
        );
    }

    pub(super) fn start_loaded_navigation_sample(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        if !self.waveform.current.has_loaded_sample()
            || self.waveform.current.path() != Path::new(path)
        {
            return false;
        }

        self.maybe_open_audio_player(context);
        let file_name = self.waveform.current.file_name();
        self.start_cached_sample_playback(&file_name, LOADED_NAVIGATION_OUTCOMES, started_at);
        true
    }

    fn start_cached_sample_playback(
        &mut self,
        file_name: &str,
        outcomes: CachedPlaybackOutcomes,
        started_at: Instant,
    ) {
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                self.ui.status.sample = format!("Playing {file_name}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(file_name),
                    outcomes.playing,
                    started_at,
                    None,
                );
            }
            Err(err) if self.audio.pending_playback_start.is_some() => {
                self.ui.status.sample = format!("Playing {file_name} when audio output is ready");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(file_name),
                    outcomes.pending,
                    started_at,
                    Some(&err),
                );
            }
            Err(err) => {
                self.ui.status.sample = format!("Loaded {file_name} | playback unavailable: {err}");
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(file_name),
                    outcomes.error,
                    started_at,
                    Some(&err),
                );
            }
        }
    }
}
