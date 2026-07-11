use super::*;

impl NativeAppState {
    pub(in super::super) fn start_playback_ready_instant_audition(
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
            stream_policy: PlaybackRuntimeStreamPolicy::full(),
            volume: self.audio.volume,
            playback_gain: 1.0,
            playback_gain_normalization: self
                .audio
                .normalized_audition_enabled
                .then(|| PlaybackRuntimeGainNormalization::new(0.0, 1.0)),
            replace_policy: PlaybackRuntimeReplacePolicy::FadeOutPrevious,
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
        self.audio.current_playback_span = Some((0.0, 1.0));
        let origin = self.runtime_playback_origin_for_path(path.as_str());
        let session_request = SamplePlaybackRequest::waveform(
            path,
            (0.0, 1.0),
            SamplePlaybackIntent::ExplicitPlayback,
            origin,
            SamplePlaybackHistory::Record,
        );
        self.audio
            .start_sample_playback_session(session_request, request_id, "decoded_samples");
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
        let preview_handoff_start_ratio = self.preview_slice_full_sample_handoff_ratio(path);
        let replace_policy = if preview_handoff_start_ratio.is_some() {
            PlaybackRuntimeReplacePolicy::ClearPrevious
        } else {
            PlaybackRuntimeReplacePolicy::FadeOutPrevious
        };
        self.start_current_sample_autoplay_with_replace_policy(
            path,
            file_name,
            preview_handoff_start_ratio.unwrap_or(0.0),
            replace_policy,
            started_at,
            context,
        );
    }

    pub(in crate::native_app::audio) fn start_current_sample_autoplay_with_replace_policy(
        &mut self,
        path: &str,
        file_name: &str,
        start_ratio: f32,
        replace_policy: PlaybackRuntimeReplacePolicy,
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
        if self.library.folder_browser.selected_file_id() == Some(path) {
            self.background.settled_sample_promotion_task.cancel();
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
            start_ratio,
            replace_policy,
            started_at,
            context,
        );
        self.schedule_next_starmap_audition_hit(context);
    }

    fn start_cached_sample_playback(
        &mut self,
        file_name: &str,
        outcomes: CachedPlaybackOutcomes,
        start_ratio: f32,
        replace_policy: PlaybackRuntimeReplacePolicy,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let playback_started_at = Instant::now();
        match self.start_current_full_sample_runtime_playback(start_ratio, replace_policy) {
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
                self.record_selected_sample_last_played(context);
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

    fn start_current_full_sample_runtime_playback(
        &mut self,
        start_ratio: f32,
        replace_policy: PlaybackRuntimeReplacePolicy,
    ) -> Result<(), String> {
        let current_path = self.waveform.current.path().display().to_string();
        let origin = self.runtime_playback_origin_for_path(current_path.as_str());
        self.submit_current_full_sample_runtime_playback(FullSamplePlaybackOptions {
            start_ratio,
            replace_policy,
            origin,
            history: SamplePlaybackHistory::Record,
            show_start_marker: true,
            record_history: true,
        })
    }

    fn submit_current_full_sample_runtime_playback(
        &mut self,
        options: FullSamplePlaybackOptions,
    ) -> Result<(), String> {
        if !self.waveform.current.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        let current_path = self.waveform.current.path().display().to_string();
        let start_ratio = options.start_ratio.clamp(0.0, 0.999);
        self.prepare_playback_mode_for_loaded_sample();
        if self.audio.playback_runtime.is_none() {
            let intent = PlaybackIntent::new(start_ratio, 1.0);
            self.audio.pending_playback_start = Some(if options.record_history {
                PendingPlaybackStart::record(intent)
            } else {
                PendingPlaybackStart::skip_history(intent)
            });
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
                    offset: f64::from(start_ratio),
                }
            } else {
                PlaybackRuntimeMode::OneShot {
                    start: f64::from(start_ratio),
                    end: 1.0,
                }
            },
            stream_policy: PlaybackRuntimeStreamPolicy::full(),
            volume: self.audio.volume,
            playback_gain,
            playback_gain_normalization,
            replace_policy: options.replace_policy,
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, start_ratio),
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
        if options.show_start_marker {
            self.waveform.current.start_playback(start_ratio);
        } else {
            self.waveform
                .current
                .start_playback_without_marker(start_ratio);
        }
        self.audio.current_playback_span = Some((0.0, 1.0));
        let source_kind = self.current_waveform_runtime_source_kind();
        let session_request = SamplePlaybackRequest::waveform(
            current_path.clone(),
            (0.0, 1.0),
            SamplePlaybackIntent::ExplicitPlayback,
            options.origin,
            options.history,
        )
        .with_start_marker(options.show_start_marker);
        self.audio
            .start_sample_playback_session(session_request, request_id, source_kind);
        if options.record_history {
            self.record_current_playback_history(0.0, 1.0);
        }
        Ok(())
    }

    pub(in crate::native_app::audio) fn preview_slice_full_sample_handoff_ratio(
        &self,
        path: &str,
    ) -> Option<f32> {
        if !self.audio.active_sample_playback_is_preview(path)
            || !self.waveform.current.has_loaded_sample()
            || self.waveform.current.path() != Path::new(path)
        {
            return None;
        }
        preview_slice_full_sample_handoff_ratio(
            self.waveform.current.duration_seconds(),
            self.audio.playback_progress.elapsed,
            self.audio.playback_progress.progress,
        )
    }
}

fn preview_slice_full_sample_handoff_ratio(
    full_duration_seconds: f32,
    elapsed: Option<Duration>,
    preview_progress: Option<f32>,
) -> Option<f32> {
    if !full_duration_seconds.is_finite() || full_duration_seconds <= 0.0 {
        return None;
    }
    let heard_seconds = elapsed.map(|elapsed| elapsed.as_secs_f32()).or_else(|| {
        preview_progress.map(|progress| progress * PREVIEW_AUDITION_DURATION.as_secs_f32())
    })?;
    if !heard_seconds.is_finite() || heard_seconds <= 0.0 {
        return None;
    }
    Some((heard_seconds / full_duration_seconds).clamp(0.0, 0.999))
}
