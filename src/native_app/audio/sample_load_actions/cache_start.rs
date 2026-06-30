use radiant::prelude as ui;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use crate::native_app::{
    app::{
        GuiMessage, NativeAppState, PendingPlaybackStart, PendingRuntimePlaybackStart,
        WaveformState, emit_gui_action, sample_path_label,
    },
    audio::{
        playback::PlaybackIntent,
        sample_load_actions::{log_sample_load_timing, types::SampleLoadStrategy},
    },
};
use wavecrate::audio::{PlaybackRuntimeMode, PlaybackRuntimeRequest, PlaybackRuntimeSource};

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
        let cache_lookup_started_at = Instant::now();
        let Some(file) = self
            .waveform
            .cache
            .entries
            .get(Path::new(path))
            .map(|entry| std::sync::Arc::clone(&entry.file))
        else {
            return false;
        };
        log_sample_load_timing(
            "browser.sample_load.memory_cache.lookup",
            path,
            cache_lookup_started_at.elapsed(),
            false,
        );
        let replace_started_at = Instant::now();
        let waveform = WaveformState::from_cached_file(file);
        if autoplay
            && self.loop_playback_for_path_after_policy(path)
            && !waveform.has_loop_stable_playback_source()
        {
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path)),
                "memory_cache_requires_decoded_loop_playback",
                started_at,
                None,
            );
            return false;
        }
        let file_name = waveform.file_name();
        self.touch_cached_waveform_path(PathBuf::from(path));
        self.stop_current_sample_playback_for_load();
        self.clear_sample_loading_state();
        self.waveform.load.selection.start_cached(path);
        self.replace_waveform_deferred(waveform);
        log_sample_load_timing(
            "browser.sample_load.memory_cache.replace_waveform",
            &file_name,
            replace_started_at.elapsed(),
            false,
        );
        if self.start_pending_sample_playback(path, &file_name, started_at, context) {
            log_sample_load_timing(
                "browser.sample_load.memory_cache.total",
                &file_name,
                started_at.elapsed(),
                false,
            );
            return true;
        }
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
        let audio_open_started_at = Instant::now();
        self.maybe_open_audio_player(context);
        log_sample_load_timing(
            "browser.sample_load.memory_cache.audio_open",
            &file_name,
            audio_open_started_at.elapsed(),
            false,
        );
        self.start_cached_sample_playback(&file_name, MEMORY_CACHE_OUTCOMES, started_at, context);
        self.schedule_next_starmap_audition_hit(context);
        log_sample_load_timing(
            "browser.sample_load.memory_cache.total",
            &file_name,
            started_at.elapsed(),
            false,
        );
        true
    }

    pub(super) fn start_foreground_sample_load(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        if self.sample_load_blocked_by_normalization(path) {
            self.ui.status.sample = format!(
                "Selected {} | waiting for normalization",
                sample_path_label(path)
            );
            self.schedule_deferred_sample_load(
                path.to_owned(),
                autoplay,
                false,
                crate::native_app::audio::sample_load_actions::NORMALIZATION_SAMPLE_LOAD_RETRY_DELAY,
                "normalization",
                context,
            );
            return;
        }
        self.prepare_uncached_sample_load(path, "foreground_load_queued", started_at);
        self.start_sample_load_with_priority(
            path.to_owned(),
            autoplay,
            context,
            ui::TaskPriority::Interactive,
            SampleLoadStrategy::CacheThenDecode,
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
        if self.loop_playback_for_path_after_policy(path)
            && !self.waveform.current.has_loop_stable_playback_source()
        {
            return false;
        }

        self.maybe_open_audio_player(context);
        let file_name = self.waveform.current.file_name();
        self.start_cached_sample_playback(
            &file_name,
            LOADED_NAVIGATION_OUTCOMES,
            started_at,
            context,
        );
        self.schedule_next_starmap_audition_hit(context);
        true
    }

    fn start_cached_sample_playback(
        &mut self,
        file_name: &str,
        outcomes: CachedPlaybackOutcomes,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let playback_started_at = Instant::now();
        match self.start_current_full_sample_runtime_playback() {
            Ok(()) => {
                self.record_selected_sample_last_played(context);
                log_sample_load_timing(
                    "browser.sample_load.cached_playback.submit",
                    file_name,
                    playback_started_at.elapsed(),
                    false,
                );
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

    fn start_current_full_sample_runtime_playback(&mut self) -> Result<(), String> {
        if !self.waveform.current.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        self.prepare_playback_mode_for_loaded_sample();
        if self.audio.playback_runtime.is_none() {
            self.audio.pending_playback_start =
                Some(PendingPlaybackStart::record(PlaybackIntent::new(0.0, 1.0)));
            if self.background.audio_open.active().is_some() {
                return Ok(());
            }
            return Err(String::from("Audio output is starting"));
        }
        let runtime = self
            .audio
            .playback_runtime
            .as_ref()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        let duration = self.waveform.current.duration_seconds();
        let source = if let Some(samples) = self.waveform.current.playback_samples() {
            PlaybackRuntimeSource::DecodedSamples {
                audio_bytes: self.waveform.current.audio_bytes(),
                samples,
                duration,
                sample_rate: self.waveform.current.sample_rate(),
                channels: self.waveform.current.channels(),
            }
        } else if let Some(cache_file) = self.waveform.current.playback_cache_file() {
            PlaybackRuntimeSource::InterleavedF32File {
                path: cache_file.path,
                sample_count: cache_file.sample_count,
                duration,
                sample_rate: self.waveform.current.sample_rate(),
                channels: self.waveform.current.channels(),
            }
        } else if let Some(path) = self.waveform.current.playback_source_file() {
            PlaybackRuntimeSource::AudioFile {
                path,
                duration,
                sample_rate: self.waveform.current.sample_rate(),
                channels: self.waveform.current.channels(),
            }
        } else {
            PlaybackRuntimeSource::AudioBytes {
                data: self.waveform.current.audio_bytes(),
                duration,
                sample_rate: self.waveform.current.sample_rate(),
                channels: self.waveform.current.channels(),
            }
        };
        let request = PlaybackRuntimeRequest {
            source,
            mode: if self.audio.loop_playback {
                PlaybackRuntimeMode::Looped {
                    start: 0.0,
                    end: 1.0,
                    offset: 0.0,
                }
            } else {
                PlaybackRuntimeMode::OneShot {
                    start: 0.0,
                    end: 1.0,
                }
            },
            volume: self.audio.volume,
            playback_gain: 1.0,
            playback_gain_normalization: self.playback_gain_normalization_for_span(0.0, 1.0),
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        let request_id = runtime
            .try_play(request)
            .map_err(|err| format!("submit playback request: {err:?}"))?;
        self.waveform.current.start_playback(0.0);
        let path = self.waveform.current.path().display().to_string();
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart {
            id: request_id,
            path,
            span: (0.0, 1.0),
            show_start_marker: true,
        });
        self.record_current_playback_history(0.0, 1.0);
        Ok(())
    }
}
