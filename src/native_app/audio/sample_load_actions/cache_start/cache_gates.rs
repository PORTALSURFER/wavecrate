use super::*;

impl NativeAppState {
    pub(in crate::native_app) fn remember_starmap_last_played_sample(&mut self, path: &str) {
        self.ui.chrome.starmap_audition_queue.last_played_file_id = Some(path.to_owned());
        self.ui.chrome.starmap_audition_queue.last_played_session = self
            .audio
            .sample_playback_session
            .as_ref()
            .filter(|session| session.request.path == path)
            .cloned();
    }

    pub(in crate::native_app::audio::sample_load_actions) fn start_fast_path_audition(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        options: FastAuditionOptions,
    ) -> InstantAuditionOutcome {
        let decision_started_at = starmap_telemetry::stage_timer();
        for probe in fast_audition_probe_order(options) {
            match probe {
                FastAuditionProbe::PreviewCache => {
                    if self.start_preview_cache_instant_audition(path, context, started_at, options)
                    {
                        record_fast_audition_decision(
                            path,
                            options,
                            Some(probe),
                            InstantAuditionOutcome::Started,
                            decision_started_at,
                        );
                        return InstantAuditionOutcome::Started;
                    }
                }
                FastAuditionProbe::PersistedCache => {
                    let persisted = self.start_persisted_cache_instant_audition_with_options(
                        path,
                        context,
                        started_at,
                        options.allow_sidecar_lookup,
                        options.record_history,
                        options.origin,
                        options.replace_policy,
                    );
                    if persisted.uses_ready_source() {
                        record_fast_audition_decision(
                            path,
                            options,
                            Some(probe),
                            persisted,
                            decision_started_at,
                        );
                        return persisted;
                    }
                }
                FastAuditionProbe::FileBackedWav => {
                    if options.allow_file_backed_source
                        && self.start_file_backed_wav_instant_audition_with_options(
                            path,
                            context,
                            started_at,
                            options.record_history,
                            options.origin,
                            options.replace_policy,
                        )
                    {
                        record_fast_audition_decision(
                            path,
                            options,
                            Some(probe),
                            InstantAuditionOutcome::Started,
                            decision_started_at,
                        );
                        return InstantAuditionOutcome::Started;
                    }
                }
                FastAuditionProbe::PreviewDecode => {
                    if self.preview_audition_decode_needed(path, options) {
                        self.queue_preview_audition_decode(path.to_owned(), started_at, context);
                        record_fast_audition_decision(
                            path,
                            options,
                            Some(probe),
                            InstantAuditionOutcome::AudioPending,
                            decision_started_at,
                        );
                        return InstantAuditionOutcome::AudioPending;
                    }
                }
            }
        }
        record_fast_audition_decision(
            path,
            options,
            None,
            InstantAuditionOutcome::Unavailable,
            decision_started_at,
        );
        InstantAuditionOutcome::Unavailable
    }

    fn preview_audition_decode_needed(&self, path: &str, options: FastAuditionOptions) -> bool {
        options.queue_preview_decode
            && preview_audition_can_decode(path)
            && self
                .waveform
                .cache
                .preview_audition_decode_needed(Path::new(path))
    }

    pub(in crate::native_app::audio::sample_load_actions) fn start_memory_cached_sample(
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

    pub(in crate::native_app::audio::sample_load_actions) fn start_foreground_sample_load_with_priority(
        &mut self,
        path: &str,
        autoplay: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        priority: ui::TaskPriority,
        outcome: &'static str,
        strategy: SampleLoadStrategy,
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
            strategy,
        );
    }

    pub(in crate::native_app::audio::sample_load_actions) fn start_loaded_navigation_sample(
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

    fn start_persisted_cache_instant_audition_with_options(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        allow_sidecar_lookup: bool,
        record_history: bool,
        origin: &'static str,
        replace_policy: PlaybackRuntimeReplacePolicy,
    ) -> InstantAuditionOutcome {
        let lookup_started_at = Instant::now();
        let descriptor = if let Some(descriptor) = self
            .waveform
            .cache
            .instant_audition_descriptors
            .get(Path::new(path))
            .cloned()
        {
            descriptor
        } else if allow_sidecar_lookup {
            if !self.loop_playback_for_path_after_policy(path)
                && should_use_file_backed_wav_decode(Path::new(path))
            {
                return InstantAuditionOutcome::Unavailable;
            }
            if let Some(descriptor) =
                load_cached_waveform_playback_descriptor_sidecar(PathBuf::from(path))
            {
                self.waveform
                    .cache
                    .mark_sample_playback_descriptor_ready(descriptor.clone());
                descriptor
            } else {
                return InstantAuditionOutcome::Unavailable;
            }
        } else {
            return InstantAuditionOutcome::Unavailable;
        };
        log_sample_load_timing(
            "browser.sample_load.persisted_descriptor.lookup",
            path,
            lookup_started_at.elapsed(),
            false,
        );
        self.prepare_playback_mode_for_path(path);
        self.maybe_open_audio_player(context);
        let Some(runtime) = self.audio.playback_runtime.clone() else {
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path)),
                "persisted_descriptor_audio_pending",
                started_at,
                None,
            );
            return InstantAuditionOutcome::AudioPending;
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
            stream_policy: PlaybackRuntimeStreamPolicy::transient_navigation(),
            volume: self.audio.volume,
            playback_gain: 1.0,
            playback_gain_normalization: self
                .audio
                .normalized_audition_enabled
                .then(|| PlaybackRuntimeGainNormalization::new(0.0, 1.0)),
            replace_policy,
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        self.cancel_replaced_starmap_foreground_load_for_fast_audition(path, origin);
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
                return InstantAuditionOutcome::Unavailable;
            }
        };
        let visibility = fast_audition_session_visibility();
        self.audio.current_playback_span =
            visibility.updates_waveform_playhead().then_some((0.0, 1.0));
        let session_request = SamplePlaybackRequest::transient(
            path.to_owned(),
            fast_audition_session_intent_for_origin(origin),
            origin,
        )
        .with_start_marker(visibility.updates_waveform_playhead());
        self.audio.start_sample_playback_session(
            session_request,
            request_id,
            "interleaved_f32_file",
        );
        self.ui.status.sample = format!("Playing {}", sample_path_label(path));
        if record_history {
            self.record_sample_last_played(path.to_owned(), context);
        }
        self.maybe_schedule_starmap_audition_promotion(path, origin, context);
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
        InstantAuditionOutcome::Started
    }

    fn start_file_backed_wav_instant_audition_with_options(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        record_history: bool,
        origin: &'static str,
        replace_policy: PlaybackRuntimeReplacePolicy,
    ) -> bool {
        if self.loop_playback_for_path_after_policy(path) {
            return false;
        }
        let lookup_started_at = Instant::now();
        if !should_use_file_backed_wav_decode(Path::new(path)) {
            return false;
        }
        log_sample_load_timing(
            "browser.sample_load.file_backed_wav.lookup",
            path,
            lookup_started_at.elapsed(),
            false,
        );
        self.prepare_playback_mode_for_path(path);
        self.maybe_open_audio_player(context);
        let Some(runtime) = self.audio.playback_runtime.clone() else {
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path)),
                "file_backed_wav_audio_pending",
                started_at,
                None,
            );
            return false;
        };
        let playback_started_at = Instant::now();
        let source = PlaybackRuntimeSource::WavFile {
            path: PathBuf::from(path),
        };
        let request = PlaybackRuntimeRequest {
            source,
            mode: PlaybackRuntimeMode::OneShot {
                start: 0.0,
                end: 1.0,
            },
            stream_policy: PlaybackRuntimeStreamPolicy::transient_navigation(),
            volume: self.audio.volume,
            playback_gain: 1.0,
            playback_gain_normalization: self
                .audio
                .normalized_audition_enabled
                .then(|| PlaybackRuntimeGainNormalization::new(0.0, 1.0)),
            replace_policy,
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        self.cancel_replaced_starmap_foreground_load_for_fast_audition(path, origin);
        let request_id = match runtime.try_play(request) {
            Ok(request_id) => request_id,
            Err(err) => {
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&sample_path_label(path)),
                    "file_backed_wav_playback_error",
                    started_at,
                    Some(&format!("submit playback request: {err:?}")),
                );
                return false;
            }
        };
        self.audio.current_playback_span = None;
        let session_request = SamplePlaybackRequest::transient(
            path.to_owned(),
            SamplePlaybackIntent::TransientNavigation,
            origin,
        )
        .with_start_marker(false);
        self.audio
            .start_sample_playback_session(session_request, request_id, "audio_file");
        self.ui.status.sample = format!("Playing {}", sample_path_label(path));
        if record_history {
            self.record_sample_last_played(path.to_owned(), context);
        }
        self.maybe_schedule_starmap_audition_promotion(path, origin, context);
        log_sample_load_timing(
            "browser.sample_load.file_backed_wav.playback_submit",
            path,
            playback_started_at.elapsed(),
            false,
        );
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path)),
            "file_backed_wav_playback_started",
            started_at,
            None,
        );
        true
    }

    fn start_preview_cache_instant_audition(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        options: FastAuditionOptions,
    ) -> bool {
        let lookup_started_at = Instant::now();
        let Some(clip) = self.waveform.cache.preview_audition_clip(Path::new(path)) else {
            return false;
        };
        log_sample_load_timing(
            "browser.sample_load.preview_audition.lookup",
            path,
            lookup_started_at.elapsed(),
            false,
        );
        self.start_preview_clip_instant_audition(clip, context, started_at, options)
    }

    pub(super) fn start_preview_clip_instant_audition(
        &mut self,
        clip: PreviewAuditionClip,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        options: FastAuditionOptions,
    ) -> bool {
        let path = clip.path.display().to_string();
        if self.loop_playback_for_path_after_policy(path.as_str()) {
            return false;
        }
        self.prepare_playback_mode_for_path(path.as_str());
        self.maybe_open_audio_player(context);
        let Some(runtime) = self.audio.playback_runtime.clone() else {
            emit_gui_action(
                "browser.select_sample",
                Some("browser"),
                Some(&sample_path_label(path.as_str())),
                "preview_audition_audio_pending",
                started_at,
                None,
            );
            return false;
        };
        let playback_started_at = Instant::now();
        let duration = clip.duration_seconds();
        let playback_gain =
            preview_clip_playback_gain(&clip, self.audio.normalized_audition_enabled);
        let source = PlaybackRuntimeSource::DecodedSamples {
            audio_bytes: Arc::from([]),
            samples: clip.samples,
            duration,
            sample_rate: clip.sample_rate,
            channels: clip.channels,
        };
        let request = PlaybackRuntimeRequest {
            source,
            mode: PlaybackRuntimeMode::OneShot {
                start: 0.0,
                end: 1.0,
            },
            stream_policy: PlaybackRuntimeStreamPolicy::transient_navigation(),
            volume: self.audio.volume,
            playback_gain,
            playback_gain_normalization: None,
            replace_policy: options.replace_policy,
            edit_fade: None,
            metronome: self.playback_metronome_config_for_span(0.0, 1.0, 0.0),
        };
        self.cancel_replaced_starmap_foreground_load_for_fast_audition(
            path.as_str(),
            options.origin,
        );
        let request_id = match runtime.try_play(request) {
            Ok(request_id) => request_id,
            Err(err) => {
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&sample_path_label(path.as_str())),
                    "preview_audition_playback_error",
                    started_at,
                    Some(&format!("submit playback request: {err:?}")),
                );
                return false;
            }
        };
        self.audio.current_playback_span = None;
        let session_request = SamplePlaybackRequest::transient(
            path.clone(),
            fast_audition_session_intent(options),
            options.origin,
        );
        self.audio
            .start_sample_playback_session(session_request, request_id, "preview_samples");
        self.ui.status.sample = format!("Playing {}", sample_path_label(path.as_str()));
        if options.record_history {
            self.record_sample_last_played(path.clone(), context);
        }
        self.maybe_schedule_starmap_audition_promotion(path.as_str(), options.origin, context);
        log_sample_load_timing(
            "browser.sample_load.preview_audition.playback_submit",
            path.as_str(),
            playback_started_at.elapsed(),
            false,
        );
        emit_gui_action(
            "browser.select_sample",
            Some("browser"),
            Some(&sample_path_label(path.as_str())),
            "preview_audition_playback_started",
            started_at,
            None,
        );
        true
    }

    pub(super) fn maybe_schedule_starmap_audition_promotion(
        &mut self,
        path: &str,
        origin: &'static str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if origin == "starmap_drag" {
            self.remember_starmap_last_played_sample(path);
            self.schedule_starmap_audition_promotion(path.to_owned(), context);
        }
    }

    pub(in crate::native_app) fn cancel_replaced_starmap_foreground_load_for_fast_audition(
        &mut self,
        path: &str,
        origin: &'static str,
    ) {
        if origin != "starmap_drag" {
            return;
        }
        let had_validation = self
            .background
            .sample_load_validation_task
            .active()
            .is_some();
        let had_preview_decode = self.background.preview_audition_task.active().is_some();
        self.background.sample_load_validation_task.cancel();
        self.background.preview_audition_task.cancel();
        let foreground_load_for_other_path = self
            .waveform
            .load
            .selection
            .selected_path
            .as_deref()
            .is_some_and(|selected| selected != path);
        if !foreground_load_for_other_path {
            return;
        }
        self.cancel_inflight_sample_load();
        self.waveform.load.selection.selected_path = None;
        self.waveform.load.label = None;
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        tracing::debug!(
            target: "perf::starmap_drag",
            event = "fast_audition.cancel_replaced_load",
            path = %path,
            had_validation,
            had_preview_decode,
            "Cancelled stale starmap foreground load before fast audition"
        );
    }

    fn queue_preview_audition_decode(
        &mut self,
        path: String,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.loop_playback_for_path_after_policy(path.as_str()) {
            return;
        }
        context
            .business()
            .interactive(PREVIEW_AUDITION_TASK_NAME)
            .latest(&mut self.background.preview_audition_task)
            .run(
                move |worker_context| {
                    let clip = if worker_context.is_cancelled() {
                        Err(String::from("cancelled"))
                    } else {
                        decode_wav_preview_clip(
                            PathBuf::from(path.as_str()),
                            PREVIEW_AUDITION_DURATION,
                        )
                    };
                    PreviewAuditionResult { path, clip }
                },
                move |completion| GuiMessage::PreviewAuditionDecoded {
                    completion,
                    started_at,
                },
            );
    }
}
