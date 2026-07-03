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
    waveform::{WaveformPlaybackReady, load_cached_waveform_playback_descriptor_sidecar},
};
use wavecrate::audio::{
    PlaybackRuntimeGainNormalization, PlaybackRuntimeMode, PlaybackRuntimeRequest,
    PlaybackRuntimeSource,
};

struct CachedPlaybackOutcomes {
    playing: &'static str,
    pending: &'static str,
    error: &'static str,
}

const SAMPLE_AUTOPLAY_OUTCOMES: CachedPlaybackOutcomes = CachedPlaybackOutcomes {
    playing: "autoplay_started",
    pending: "autoplay_pending",
    error: "autoplay_error",
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
        self.log_sample_identity_waveform_checkpoint(
            "browser.sample_load.memory_cache_candidate",
            "start_memory_cached_sample",
            Some(Path::new(path)),
            &waveform,
            Some(if autoplay { "autoplay" } else { "load_only" }),
        );
        self.log_sample_identity_checkpoint(
            "browser.sample_load.memory_cache_before_replace",
            "start_memory_cached_sample",
            Some(Path::new(path)),
            Some(if autoplay { "autoplay" } else { "load_only" }),
        );
        self.replace_waveform_deferred(waveform);
        self.log_sample_identity_checkpoint(
            "browser.sample_load.memory_cache_after_replace",
            "start_memory_cached_sample",
            Some(Path::new(path)),
            Some(if autoplay { "autoplay" } else { "load_only" }),
        );
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
        self.start_current_sample_autoplay(path, &file_name, started_at, context);
        log_sample_load_timing(
            "browser.sample_load.memory_cache.total",
            &file_name,
            started_at.elapsed(),
            false,
        );
        true
    }

    pub(super) fn start_foreground_sample_load_with_priority(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        priority: ui::TaskPriority,
        outcome: &'static str,
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
        self.prepare_uncached_sample_load(path, outcome, started_at);
        self.start_sample_load_with_priority(
            path.to_owned(),
            autoplay,
            context,
            priority,
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

        let file_name = self.waveform.current.file_name();
        self.start_current_sample_autoplay(path, &file_name, started_at, context);
        true
    }

    pub(super) fn start_persisted_cache_instant_audition(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        let lookup_started_at = Instant::now();
        let descriptor = if let Some(descriptor) = self
            .waveform
            .cache
            .instant_audition_descriptors
            .get(Path::new(path))
            .cloned()
        {
            descriptor
        } else if let Some(descriptor) =
            load_cached_waveform_playback_descriptor_sidecar(PathBuf::from(path))
        {
            self.waveform
                .cache
                .mark_sample_playback_descriptor_ready(descriptor.clone());
            descriptor
        } else {
            return false;
        };
        log_sample_load_timing(
            "browser.sample_load.persisted_descriptor.lookup",
            path,
            lookup_started_at.elapsed(),
            false,
        );
        self.prepare_playback_mode_for_path(path);
        self.maybe_open_audio_player(context);
        let Some(runtime) = self.audio.playback_runtime.as_ref() else {
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path)),
                "persisted_descriptor_audio_pending",
                started_at,
                None,
            );
            return false;
        };
        let playback_started_at = Instant::now();
        let duration = descriptor.duration_seconds();
        let source = PlaybackRuntimeSource::InterleavedF32File {
            path: descriptor.cache_file.path,
            sample_count: descriptor.cache_file.sample_count,
            duration,
            sample_rate: descriptor.sample_rate,
            channels: descriptor.channels,
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
            playback_gain_normalization: self
                .audio
                .normalized_audition_enabled
                .then(|| PlaybackRuntimeGainNormalization::new(0.0, 1.0)),
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        let request_id = match runtime.try_play(request) {
            Ok(request_id) => request_id,
            Err(err) => {
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&sample_path_label(path)),
                    "persisted_descriptor_playback_error",
                    started_at,
                    Some(&format!("submit playback request: {err:?}")),
                );
                return false;
            }
        };
        self.audio.early_sample_playback_path = Some(path.to_owned());
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart {
            id: request_id,
            path: path.to_owned(),
            span: (0.0, 1.0),
            show_start_marker: true,
        });
        self.ui.status.sample = format!("Playing {}", sample_path_label(path));
        self.record_sample_last_played(path.to_owned(), context);
        log_sample_load_timing(
            "browser.sample_load.persisted_descriptor.playback_submit",
            path,
            playback_started_at.elapsed(),
            false,
        );
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path)),
            "persisted_descriptor_playback_started",
            started_at,
            None,
        );
        true
    }

    pub(super) fn start_playback_ready_instant_audition(
        &mut self,
        ready: WaveformPlaybackReady,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        let path = ready.path.display().to_string();
        let label = sample_path_label(path.as_str());
        self.prepare_playback_mode_for_path(path.as_str());
        self.maybe_open_audio_player(context);
        let Some(runtime) = self.audio.playback_runtime.as_ref() else {
            return false;
        };
        let playback_started_at = Instant::now();
        let duration = ready.frames as f32 / ready.sample_rate.max(1) as f32;
        let source = PlaybackRuntimeSource::DecodedSamples {
            audio_bytes: ready.audio_bytes,
            samples: ready.playback_samples,
            duration,
            sample_rate: ready.sample_rate,
            channels: ready.channels,
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
            playback_gain_normalization: self
                .audio
                .normalized_audition_enabled
                .then(|| PlaybackRuntimeGainNormalization::new(0.0, 1.0)),
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        let request_id = match runtime.try_play(request) {
            Ok(request_id) => request_id,
            Err(err) => {
                emit_gui_action(
                    "browser.sample_load.playback_ready",
                    Some("browser"),
                    Some(&label),
                    "instant_playback_error",
                    started_at,
                    Some(&format!("submit playback request: {err:?}")),
                );
                return false;
            }
        };
        self.audio.early_sample_playback_path = Some(path.clone());
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart {
            id: request_id,
            path,
            span: (0.0, 1.0),
            show_start_marker: true,
        });
        self.ui.status.sample = format!("Playing {label}");
        log_sample_load_timing(
            "browser.sample_load.playback_ready.playback_submit",
            &label,
            playback_started_at.elapsed(),
            false,
        );
        true
    }

    pub(in crate::native_app::audio) fn start_current_sample_autoplay(
        &mut self,
        path: &str,
        file_name: &str,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.waveform.current.has_loaded_sample()
            || self.waveform.current.path() != Path::new(path)
        {
            emit_gui_action(
                "browser.sample_load.autoplay",
                Some("browser"),
                Some(file_name),
                "stale",
                started_at,
                None,
            );
            return;
        }
        let audio_open_started_at = Instant::now();
        self.maybe_open_audio_player(context);
        log_sample_load_timing(
            "browser.sample_load.autoplay.audio_open",
            file_name,
            audio_open_started_at.elapsed(),
            false,
        );
        self.start_cached_sample_playback(
            &file_name,
            SAMPLE_AUTOPLAY_OUTCOMES,
            started_at,
            context,
        );
        self.schedule_next_starmap_audition_hit(context);
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
        let (playback_gain, playback_gain_normalization) =
            self.runtime_playback_gain_for_span(0.0, 1.0);
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
            playback_gain,
            playback_gain_normalization,
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        self.log_sample_identity_checkpoint(
            "playback.runtime.full_sample_request_built",
            "start_current_full_sample_runtime_playback",
            Some(&self.waveform.current.path()),
            Some(if self.audio.loop_playback {
                "looped"
            } else {
                "one_shot"
            }),
        );
        let runtime = self
            .audio
            .playback_runtime
            .as_ref()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
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
