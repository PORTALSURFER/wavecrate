use radiant::prelude as ui;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use crate::native_app::{
    app::{
        GuiMessage, NativeAppState, PendingPlaybackStart, PendingRuntimePlaybackStart,
        PreviewAuditionResult, PreviewAuditionWarmResult, SampleBrowserDisplayMode, WaveformState,
        emit_gui_action, sample_path_label,
    },
    audio::{
        playback::PlaybackIntent,
        sample_load_actions::{log_sample_load_timing, types::SampleLoadStrategy},
    },
    starmap_audition_telemetry as starmap_telemetry,
    waveform::{
        PreviewAuditionClip, WaveformPlaybackReady, decode_wav_preview_clip,
        load_cached_waveform_playback_descriptor_sidecar,
    },
    waveform::{file_backed_wav_playback_descriptor, should_use_file_backed_wav_decode},
};
use wavecrate::audio::{
    PlaybackRuntimeGainNormalization, PlaybackRuntimeMode, PlaybackRuntimeReplacePolicy,
    PlaybackRuntimeRequest, PlaybackRuntimeSource,
};

struct CachedPlaybackOutcomes {
    playing: &'static str,
    pending: &'static str,
    error: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InstantAuditionOutcome {
    Started,
    AudioPending,
    Unavailable,
}

impl InstantAuditionOutcome {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::AudioPending => "audio_pending",
            Self::Unavailable => "unavailable",
        }
    }

    pub(super) fn uses_ready_source(self) -> bool {
        matches!(self, Self::Started | Self::AudioPending)
    }
}

const SAMPLE_AUTOPLAY_OUTCOMES: CachedPlaybackOutcomes = CachedPlaybackOutcomes {
    playing: "autoplay_started",
    pending: "autoplay_pending",
    error: "autoplay_error",
};
const PREVIEW_AUDITION_TASK_NAME: &str = "gui-preview-audition-decode";
const PREVIEW_AUDITION_WARM_TASK_NAME: &str = "gui-preview-audition-warm";
const PREVIEW_AUDITION_DURATION: Duration = Duration::from_millis(220);
const PREVIEW_AUDITION_WARM_BATCH: usize = 24;
const PREVIEW_AUDITION_LIST_VIEW_BUDGET: usize = PREVIEW_AUDITION_WARM_BATCH * 2;
const PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD: usize = 96;
const PREVIEW_AUDITION_STARMAP_VIEW_BUDGET: usize = PREVIEW_AUDITION_WARM_BATCH * 2;
const PREVIEW_AUDITION_STARMAP_VIEWPORT_PAD: f32 = 0.08;

#[derive(Clone, Copy, Debug)]
pub(super) struct FastAuditionOptions {
    pub(super) origin: &'static str,
    pub(super) record_history: bool,
    pub(super) allow_sidecar_lookup: bool,
    pub(super) queue_preview_decode: bool,
    pub(super) prefer_preview_decode: bool,
    pub(super) allow_file_backed_probe: bool,
    pub(super) replace_policy: PlaybackRuntimeReplacePolicy,
}

impl FastAuditionOptions {
    pub(super) fn starmap_drag() -> Self {
        Self {
            origin: "starmap_drag",
            record_history: false,
            allow_sidecar_lookup: false,
            queue_preview_decode: true,
            prefer_preview_decode: true,
            allow_file_backed_probe: false,
            replace_policy: PlaybackRuntimeReplacePolicy::ClearPrevious,
        }
    }

    pub(super) fn instant_navigation() -> Self {
        Self {
            origin: "instant_audition",
            record_history: true,
            allow_sidecar_lookup: false,
            queue_preview_decode: true,
            prefer_preview_decode: true,
            allow_file_backed_probe: false,
            replace_policy: PlaybackRuntimeReplacePolicy::ClearPrevious,
        }
    }

    pub(super) fn preview_decode_completion(origin: &'static str, record_history: bool) -> Self {
        Self {
            origin,
            record_history,
            allow_sidecar_lookup: false,
            queue_preview_decode: false,
            prefer_preview_decode: false,
            allow_file_backed_probe: false,
            replace_policy: PlaybackRuntimeReplacePolicy::ClearPrevious,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FastAuditionProbe {
    PreviewCache,
    PersistedCache,
    FileBackedWav,
    PreviewDecode,
}

#[derive(Debug, Default)]
struct PreviewAuditionWarmPlan {
    paths: Vec<String>,
    starmap_signature: Option<u64>,
}

fn fast_audition_probe_order(options: FastAuditionOptions) -> [FastAuditionProbe; 4] {
    if options.prefer_preview_decode {
        [
            FastAuditionProbe::PreviewCache,
            FastAuditionProbe::PersistedCache,
            FastAuditionProbe::PreviewDecode,
            FastAuditionProbe::FileBackedWav,
        ]
    } else {
        [
            FastAuditionProbe::PreviewCache,
            FastAuditionProbe::PersistedCache,
            FastAuditionProbe::FileBackedWav,
            FastAuditionProbe::PreviewDecode,
        ]
    }
}

impl NativeAppState {
    pub(super) fn start_fast_path_audition(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        options: FastAuditionOptions,
    ) -> InstantAuditionOutcome {
        for probe in fast_audition_probe_order(options) {
            match probe {
                FastAuditionProbe::PreviewCache => {
                    if self.start_preview_cache_instant_audition(path, context, started_at, options)
                    {
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
                        return persisted;
                    }
                }
                FastAuditionProbe::FileBackedWav => {
                    if options.allow_file_backed_probe
                        && self.start_file_backed_wav_instant_audition_with_options(
                            path,
                            context,
                            started_at,
                            options.record_history,
                            options.origin,
                            options.replace_policy,
                        )
                    {
                        return InstantAuditionOutcome::Started;
                    }
                }
                FastAuditionProbe::PreviewDecode => {
                    if self.preview_audition_decode_needed(path, options) {
                        self.queue_preview_audition_decode(path.to_owned(), started_at, context);
                        return InstantAuditionOutcome::AudioPending;
                    }
                }
            }
        }
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
        self.audio.early_sample_playback_path = Some(path.to_owned());
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            path.to_owned(),
            (0.0, 1.0),
            true,
            origin,
            "interleaved_f32_file",
        ));
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
        let Some(descriptor) = file_backed_wav_playback_descriptor(Path::new(path)) else {
            return false;
        };
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
        let source = PlaybackRuntimeSource::AudioFile {
            path: descriptor.path,
            duration: descriptor.duration,
            sample_rate: descriptor.sample_rate,
            channels: descriptor.channels,
        };
        let request = PlaybackRuntimeRequest {
            source,
            mode: PlaybackRuntimeMode::OneShot {
                start: 0.0,
                end: 1.0,
            },
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
        self.audio.early_sample_playback_path = Some(path.to_owned());
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            path.to_owned(),
            (0.0, 1.0),
            true,
            origin,
            "audio_file",
        ));
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

    fn start_preview_clip_instant_audition(
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
        self.audio.early_sample_playback_path = Some(path.clone());
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            path.clone(),
            (0.0, 1.0),
            true,
            options.origin,
            "preview_samples",
        ));
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

    fn maybe_schedule_starmap_audition_promotion(
        &mut self,
        path: &str,
        origin: &'static str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if origin == "starmap_drag" {
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

    pub(in crate::native_app) fn maybe_start_preview_audition_warm(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.preview_audition_warm_should_yield() {
            self.background.preview_audition_warm_task.cancel();
            self.waveform.cache.cancel_preview_audition_warm_schedule();
            return;
        }
        if self
            .background
            .preview_audition_warm_task
            .active()
            .is_some()
        {
            return;
        }
        let plan = self.preview_audition_warm_candidates();
        if plan.paths.is_empty() {
            return;
        }
        let paths = plan.paths;
        self.waveform
            .cache
            .mark_preview_audition_warm_scheduled(&paths);
        if let Some(signature) = plan.starmap_signature {
            self.waveform
                .cache
                .reserve_starmap_preview_warm_budget(signature, paths.len());
        }
        let started_at = Instant::now();
        context
            .business()
            .background(PREVIEW_AUDITION_WARM_TASK_NAME)
            .latest(&mut self.background.preview_audition_warm_task)
            .run(
                move |worker_context| {
                    let scheduled_paths = paths.clone();
                    let mut attempted_paths = Vec::new();
                    let mut clips = Vec::new();
                    let mut errors = 0;
                    for path in paths {
                        if worker_context.is_cancelled() {
                            break;
                        }
                        attempted_paths.push(path.clone());
                        match decode_wav_preview_clip(
                            PathBuf::from(path.as_str()),
                            PREVIEW_AUDITION_DURATION,
                        ) {
                            Ok(clip) => clips.push(clip),
                            Err(_) => errors += 1,
                        }
                    }
                    PreviewAuditionWarmResult {
                        scheduled_paths,
                        attempted_paths,
                        clips,
                        errors,
                    }
                },
                move |completion| GuiMessage::PreviewAuditionWarmFinished {
                    completion,
                    started_at,
                },
            );
    }

    fn preview_audition_warm_should_yield(&self) -> bool {
        self.ui.chrome.starmap_audition_drag.is_some()
            || self.sample_cache_warm_should_pause_active()
            || self.playback_visual_activity_active()
    }

    fn preview_audition_warm_candidates(&mut self) -> PreviewAuditionWarmPlan {
        match self.ui.chrome.sample_browser_display {
            SampleBrowserDisplayMode::Map => self.preview_audition_warm_starmap_candidates(),
            SampleBrowserDisplayMode::List => PreviewAuditionWarmPlan {
                paths: self.preview_audition_warm_list_candidates(),
                starmap_signature: None,
            },
        }
    }

    fn preview_audition_warm_starmap_candidates(&mut self) -> PreviewAuditionWarmPlan {
        let Some(items) = self.library.folder_browser.cached_starmap_projection() else {
            return PreviewAuditionWarmPlan::default();
        };
        let selected = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned);
        let center_x = self.ui.chrome.starmap_viewport.center_x;
        let center_y = self.ui.chrome.starmap_viewport.center_y;
        let zoom = self.ui.chrome.starmap_viewport.zoom.max(f32::EPSILON);
        let signature = starmap_preview_warm_view_signature(
            self.library.folder_browser.selected_source_id(),
            items.len(),
            center_x,
            center_y,
            zoom,
        );
        let remaining_budget = self
            .waveform
            .cache
            .remaining_starmap_preview_warm_budget(signature, PREVIEW_AUDITION_STARMAP_VIEW_BUDGET);
        if remaining_budget == 0 {
            return PreviewAuditionWarmPlan {
                paths: Vec::new(),
                starmap_signature: Some(signature),
            };
        }
        let mut candidates = Vec::new();
        for item in items.iter() {
            if item.missing || !preview_audition_can_decode(&item.file_id) {
                continue;
            }
            let selected_item = selected.as_deref() == Some(item.file_id.as_str());
            if !selected_item
                && !starmap_item_in_preview_warm_viewport(item.x, item.y, center_x, center_y, zoom)
            {
                continue;
            }
            let score = if selected_item {
                -1.0
            } else {
                let dx = item.x - center_x;
                let dy = item.y - center_y;
                dx * dx + dy * dy
            };
            candidates.push((score, item.file_id.clone()));
        }
        candidates.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        let paths = candidates
            .into_iter()
            .map(|(_, path)| path)
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .take(PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD)
            .take(PREVIEW_AUDITION_WARM_BATCH.min(remaining_budget))
            .collect();
        PreviewAuditionWarmPlan {
            paths,
            starmap_signature: Some(signature),
        }
    }

    fn preview_audition_warm_list_candidates(&mut self) -> Vec<String> {
        let ordered_paths: Vec<String> = {
            use crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery;

            let visible = self
                .library
                .folder_browser
                .visible_samples(VisibleSampleQuery {
                    tags_by_file: &self.metadata.tags_by_file,
                    cached_sample_paths: &self.waveform.cache.cached_sample_paths,
                });
            Self::preview_audition_list_warm_ordered_paths(
                &visible.rows,
                self.library.folder_browser.selected_file_id(),
                PREVIEW_AUDITION_LIST_VIEW_BUDGET,
            )
        };
        ordered_paths
            .into_iter()
            .filter(|path| preview_audition_can_decode(path))
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .take(PREVIEW_AUDITION_WARM_BATCH)
            .collect()
    }

    fn preview_audition_list_warm_ordered_paths(
        rows: &[crate::native_app::sample_library::folder_browser::projection::VisibleSampleRow<
            '_,
        >],
        selected_file_id: Option<&str>,
        limit: usize,
    ) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }
        let selected_index = selected_file_id
            .and_then(|selected| rows.iter().position(|row| row.file.id == selected));
        let Some(selected_index) = selected_index else {
            return rows
                .iter()
                .filter(|row| !row.missing)
                .take(limit)
                .map(|row| row.file.id.clone())
                .collect();
        };
        let mut ordered = Vec::with_capacity(limit.min(rows.len()));
        for offset in 0..rows.len() {
            if offset == 0 {
                if let Some(row) = rows.get(selected_index).filter(|row| !row.missing) {
                    ordered.push(row.file.id.clone());
                }
            } else {
                if let Some(row) = selected_index
                    .checked_add(offset)
                    .and_then(|index| rows.get(index))
                    .filter(|row| !row.missing)
                {
                    ordered.push(row.file.id.clone());
                }
                if ordered.len() >= limit {
                    break;
                }
                if let Some(row) = selected_index
                    .checked_sub(offset)
                    .and_then(|index| rows.get(index))
                    .filter(|row| !row.missing)
                {
                    ordered.push(row.file.id.clone());
                }
            }
            if ordered.len() >= limit {
                break;
            }
        }
        ordered
    }

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
                    .mark_preview_audition_attempted(Path::new(result.path.as_str()));
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&sample_path_label(result.path.as_str())),
                    "preview_audition_error",
                    started_at,
                    Some(&error),
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
        let started = self.start_preview_clip_instant_audition(clip, context, started_at, options);
        if started
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

    fn preview_audition_decode_matches_current_target(&self, path: &str) -> bool {
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
        );
        let clip_count = result.clips.len();
        for clip in result.clips {
            self.waveform.cache.store_preview_audition_clip(clip);
        }
        let worker_elapsed = started_at.elapsed();
        let commit_elapsed = finish_started_at.elapsed();
        log_sample_load_timing(
            "browser.sample_load.preview_audition.warm_commit",
            "preview-audition-warm",
            commit_elapsed,
            false,
        );
        tracing::debug!(
            target: "wavecrate::debug::sample_load",
            event = "browser.sample_load.preview_audition.warm_finished",
            scheduled = result.scheduled_paths.len(),
            attempted = result.attempted_paths.len(),
            decoded = clip_count,
            errors = result.errors,
            worker_elapsed_ms = worker_elapsed.as_secs_f64() * 1000.0,
            commit_elapsed_ms = commit_elapsed.as_secs_f64() * 1000.0,
            "Preview audition warm finished"
        );
        if result.errors > 0 {
            tracing::debug!(
                attempted = result.attempted_paths.len(),
                decoded = clip_count,
                errors = result.errors,
                "Preview audition warm finished with decode misses"
            );
        }
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
        self.audio.early_sample_playback_path = Some(path.clone());
        self.audio.current_playback_span = Some((0.0, 1.0));
        let origin = self.runtime_playback_origin_for_path(path.as_str());
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            path,
            (0.0, 1.0),
            true,
            origin,
            "decoded_samples",
        ));
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
            replace_policy: PlaybackRuntimeReplacePolicy::FadeOutPrevious,
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
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            path.clone(),
            (0.0, 1.0),
            true,
            self.runtime_playback_origin_for_path(path.as_str()),
            self.current_waveform_runtime_source_kind(),
        ));
        self.record_current_playback_history(0.0, 1.0);
        Ok(())
    }
}

fn preview_audition_can_decode(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("wav") || extension.eq_ignore_ascii_case("wave")
        })
}

fn preview_clip_playback_gain(
    clip: &PreviewAuditionClip,
    normalized_audition_enabled: bool,
) -> f32 {
    if normalized_audition_enabled && clip.normalized_gain.is_finite() && clip.normalized_gain > 0.0
    {
        clip.normalized_gain
    } else {
        1.0
    }
}

fn starmap_item_in_preview_warm_viewport(
    item_x: f32,
    item_y: f32,
    center_x: f32,
    center_y: f32,
    zoom: f32,
) -> bool {
    let normalized_x = (item_x - center_x) * zoom + 0.5;
    let normalized_y = (item_y - center_y) * zoom + 0.5;
    let min = -PREVIEW_AUDITION_STARMAP_VIEWPORT_PAD;
    let max = 1.0 + PREVIEW_AUDITION_STARMAP_VIEWPORT_PAD;
    (min..=max).contains(&normalized_x) && (min..=max).contains(&normalized_y)
}

fn starmap_preview_warm_view_signature(
    source_id: &str,
    item_count: usize,
    center_x: f32,
    center_y: f32,
    zoom: f32,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    source_id.hash(&mut hasher);
    item_count.hash(&mut hasher);
    quantized_starmap_preview_warm_coordinate(center_x).hash(&mut hasher);
    quantized_starmap_preview_warm_coordinate(center_y).hash(&mut hasher);
    quantized_starmap_preview_warm_zoom(zoom).hash(&mut hasher);
    hasher.finish()
}

fn quantized_starmap_preview_warm_coordinate(value: f32) -> i32 {
    (value * 64.0).round() as i32
}

fn quantized_starmap_preview_warm_zoom(value: f32) -> i32 {
    (value * 16.0).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::Command;
    use std::{collections::HashSet, fs, path::PathBuf, sync::Arc, time::SystemTime};

    fn after_messages(command: Command<GuiMessage>) -> Vec<GuiMessage> {
        match command {
            Command::After { message, .. } => vec![message],
            Command::Batch(commands) => commands.into_iter().flat_map(after_messages).collect(),
            _ => Vec::new(),
        }
    }

    fn starmap_state_with_wav_files(
        file_count: usize,
    ) -> crate::native_app::test_support::state::NativeAppState {
        let source_root = tempfile::tempdir().expect("source root");
        let source_path = source_root.path().to_path_buf();
        for index in 0..file_count {
            fs::write(source_path.join(format!("sample-{index:03}.wav")), [])
                .expect("write wav placeholder");
        }
        let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
            .with_folder_browser(
                crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                    wavecrate::sample_sources::SampleSource::new(source_path),
                ]),
            )
            .build();
        state.ui.chrome.sample_browser_display = SampleBrowserDisplayMode::Map;
        crate::native_app::test_support::sample_browser::prepare_sample_browser_view(&mut state);
        state
    }

    fn list_state_with_wav_files(
        file_count: usize,
        selected_index: usize,
    ) -> (
        crate::native_app::test_support::state::NativeAppState,
        Vec<String>,
    ) {
        let source_root = tempfile::tempdir().expect("source root");
        let source_path = source_root.path().to_path_buf();
        let mut paths = Vec::new();
        for index in 0..file_count {
            let path = source_path.join(format!("sample-{index:03}.wav"));
            fs::write(&path, []).expect("write wav placeholder");
            paths.push(path.display().to_string());
        }
        let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
            .with_folder_browser(
                crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                    wavecrate::sample_sources::SampleSource::new(source_path),
                ]),
            )
            .build();
        state.ui.chrome.sample_browser_display = SampleBrowserDisplayMode::List;
        state
            .library
            .folder_browser
            .select_file(paths[selected_index].clone());
        crate::native_app::test_support::sample_browser::prepare_sample_browser_view(&mut state);
        (state, paths)
    }

    fn preview_clip_with_gain(normalized_gain: f32) -> PreviewAuditionClip {
        PreviewAuditionClip {
            path: PathBuf::from("/tmp/wavecrate-preview-gain.wav"),
            source_len: 0,
            source_modified: Some(SystemTime::UNIX_EPOCH),
            samples: Arc::from([0.25_f32, -0.5]),
            sample_rate: 44_100,
            channels: 1,
            frames: 2,
            normalized_gain,
        }
    }

    #[test]
    fn starmap_drag_fast_audition_prefers_preview_decode_before_source_file_probe() {
        assert_eq!(
            fast_audition_probe_order(FastAuditionOptions::starmap_drag()),
            [
                FastAuditionProbe::PreviewCache,
                FastAuditionProbe::PersistedCache,
                FastAuditionProbe::PreviewDecode,
                FastAuditionProbe::FileBackedWav,
            ]
        );
    }

    #[test]
    fn instant_navigation_fast_audition_prefers_preview_decode_before_source_file_probe() {
        assert_eq!(
            fast_audition_probe_order(FastAuditionOptions::instant_navigation()),
            [
                FastAuditionProbe::PreviewCache,
                FastAuditionProbe::PersistedCache,
                FastAuditionProbe::PreviewDecode,
                FastAuditionProbe::FileBackedWav,
            ]
        );
    }

    #[test]
    fn instant_navigation_fast_audition_avoids_ui_thread_sidecar_lookup() {
        assert!(
            !FastAuditionOptions::instant_navigation().allow_sidecar_lookup,
            "list and keyboard navigation should not read playback descriptor sidecars on the UI path"
        );
    }

    #[test]
    fn hot_fast_audition_options_avoid_ui_thread_source_file_probing() {
        assert!(
            !FastAuditionOptions::instant_navigation().allow_file_backed_probe,
            "list and keyboard navigation should not probe WAV headers on the UI path"
        );
        assert!(
            !FastAuditionOptions::starmap_drag().allow_file_backed_probe,
            "starmap drag playback should not probe WAV headers on the UI path"
        );
    }

    #[test]
    fn hot_fast_audition_options_clear_previous_runtime_source() {
        assert_eq!(
            FastAuditionOptions::instant_navigation().replace_policy,
            PlaybackRuntimeReplacePolicy::ClearPrevious,
            "list and keyboard navigation should not keep old preview sources fading in the mixer"
        );
        assert_eq!(
            FastAuditionOptions::starmap_drag().replace_policy,
            PlaybackRuntimeReplacePolicy::ClearPrevious,
            "starmap drag playback should replace the prior preview source immediately"
        );
    }

    #[test]
    fn preview_clip_playback_uses_precomputed_normalized_gain() {
        let clip = preview_clip_with_gain(2.5);

        assert_eq!(preview_clip_playback_gain(&clip, true), 2.5);
        assert_eq!(preview_clip_playback_gain(&clip, false), 1.0);
        assert_eq!(
            preview_clip_playback_gain(&preview_clip_with_gain(f32::NAN), true),
            1.0
        );
        assert_eq!(
            preview_clip_playback_gain(&preview_clip_with_gain(0.0), true),
            1.0
        );
    }

    #[test]
    fn list_preview_warm_stops_after_selected_neighborhood_budget() {
        let selected_index = 48;
        let (mut state, paths) = list_state_with_wav_files(96, selected_index);
        let mut warmed = HashSet::new();

        for _ in 0..(PREVIEW_AUDITION_LIST_VIEW_BUDGET / PREVIEW_AUDITION_WARM_BATCH) {
            let plan = state.preview_audition_warm_list_candidates();
            assert_eq!(plan.len(), PREVIEW_AUDITION_WARM_BATCH);
            warmed.extend(plan.iter().cloned());
            state
                .waveform
                .cache
                .mark_preview_audition_warm_scheduled(&plan);
        }

        assert_eq!(warmed.len(), PREVIEW_AUDITION_LIST_VIEW_BUDGET);
        assert!(
            warmed.contains(&paths[selected_index]),
            "list preview warming should include the selected row"
        );
        assert!(
            !warmed.contains(&paths[0]),
            "list preview warming should not crawl back to the start of a large source"
        );
        assert!(
            !warmed.contains(&paths[paths.len() - 1]),
            "list preview warming should not crawl to the end of a large source"
        );

        let exhausted_plan = state.preview_audition_warm_list_candidates();

        assert_eq!(
            exhausted_plan.len(),
            0,
            "list preview warming should stop after the selected-row neighborhood budget"
        );
    }

    #[test]
    fn legacy_preview_preference_keeps_preview_decode_before_file_backed_wav() {
        let options = FastAuditionOptions {
            origin: "test",
            record_history: false,
            allow_sidecar_lookup: false,
            queue_preview_decode: true,
            prefer_preview_decode: true,
            allow_file_backed_probe: false,
            replace_policy: PlaybackRuntimeReplacePolicy::ClearPrevious,
        };

        assert_eq!(
            fast_audition_probe_order(options),
            [
                FastAuditionProbe::PreviewCache,
                FastAuditionProbe::PersistedCache,
                FastAuditionProbe::PreviewDecode,
                FastAuditionProbe::FileBackedWav,
            ]
        );
    }

    #[test]
    fn preview_decode_completion_uses_active_starmap_target_during_drag() {
        let mut state = starmap_state_with_wav_files(2);
        let files = state
            .library
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        let active = files[0].clone();
        let selected = files[1].clone();
        state.ui.chrome.starmap_audition_drag =
            Some(crate::native_app::app::StarmapAuditionDragState {
                last_hit_file_id: Some(active.clone()),
                last_position: ui::Point::new(0.0, 0.0),
                modifiers: Default::default(),
            });
        state.ui.chrome.starmap_audition_queue.active_file_id = Some(active.clone());
        state.library.folder_browser.select_file(selected);

        assert!(
            state.preview_audition_decode_matches_current_target(active.as_str()),
            "active starmap drag target should be allowed to finish even if browser selection has already moved"
        );
    }

    #[test]
    fn preview_decode_completion_rejects_replaced_starmap_target() {
        let mut state = starmap_state_with_wav_files(2);
        let files = state
            .library
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        let stale = files[0].clone();
        let active = files[1].clone();
        state.ui.chrome.starmap_audition_drag =
            Some(crate::native_app::app::StarmapAuditionDragState {
                last_hit_file_id: Some(active.clone()),
                last_position: ui::Point::new(0.0, 0.0),
                modifiers: Default::default(),
            });
        state.ui.chrome.starmap_audition_queue.active_file_id = Some(active);
        state.library.folder_browser.select_file(stale.clone());

        assert!(
            !state.preview_audition_decode_matches_current_target(stale.as_str()),
            "stale starmap preview decode must not play just because the browser selection still points at it"
        );
    }

    fn reserve_starmap_preview_warm_plan(
        state: &mut crate::native_app::test_support::state::NativeAppState,
        plan: &PreviewAuditionWarmPlan,
    ) {
        state
            .waveform
            .cache
            .mark_preview_audition_warm_scheduled(&plan.paths);
        if let Some(signature) = plan.starmap_signature {
            state
                .waveform
                .cache
                .reserve_starmap_preview_warm_budget(signature, plan.paths.len());
        }
    }

    #[test]
    fn starmap_preview_warm_stops_after_view_budget_until_view_changes() {
        let mut state = starmap_state_with_wav_files(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET + 48);
        let mut warmed = HashSet::new();

        for _ in 0..(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET / PREVIEW_AUDITION_WARM_BATCH) {
            let plan = state.preview_audition_warm_starmap_candidates();
            assert_eq!(plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
            warmed.extend(plan.paths.iter().cloned());
            reserve_starmap_preview_warm_plan(&mut state, &plan);
        }
        assert_eq!(warmed.len(), PREVIEW_AUDITION_STARMAP_VIEW_BUDGET);

        let exhausted_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(
            exhausted_plan.paths.len(),
            0,
            "starmap preview warming should not keep crawling a dense unchanged map forever"
        );

        state.ui.chrome.starmap_viewport.center_x += 0.25;
        let changed_view_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(
            changed_view_plan.paths.len(),
            PREVIEW_AUDITION_WARM_BATCH,
            "a meaningful starmap viewport change should open a fresh warm budget"
        );
        assert!(
            changed_view_plan
                .paths
                .iter()
                .all(|path| !warmed.contains(path))
        );
    }

    #[test]
    fn starmap_preview_warm_budget_survives_selection_changes() {
        let mut state = starmap_state_with_wav_files(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET + 48);
        let files = state
            .library
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();

        for _ in 0..(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET / PREVIEW_AUDITION_WARM_BATCH) {
            let plan = state.preview_audition_warm_starmap_candidates();
            assert_eq!(plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
            reserve_starmap_preview_warm_plan(&mut state, &plan);
        }

        for selected in files.iter().take(6) {
            state.library.folder_browser.select_file(selected.clone());
            let plan = state.preview_audition_warm_starmap_candidates();
            assert_eq!(
                plan.paths.len(),
                0,
                "starmap audition selection changes must not reset the finite viewport warm budget"
            );
        }
    }

    #[test]
    fn starmap_preview_warm_ignores_offscreen_zoomed_nodes() {
        assert!(starmap_item_in_preview_warm_viewport(
            0.5, 0.5, 0.5, 0.5, 4.0
        ));
        assert!(!starmap_item_in_preview_warm_viewport(
            0.95, 0.95, 0.5, 0.5, 4.0
        ));
    }

    #[test]
    fn starmap_drag_instant_audition_schedules_stable_target_promotion() {
        let mut state =
            crate::native_app::test_support::state::NativeAppStateFixture::default().build();
        let mut context = ui::UiUpdateContext::default();

        state.maybe_schedule_starmap_audition_promotion(
            "/tmp/starmap-target.wav",
            "starmap_drag",
            &mut context,
        );
        let delayed = after_messages(context.into_command());

        assert!(delayed.iter().any(|message| matches!(
            message,
            GuiMessage::PromoteStarmapAudition {
                path,
                ..
            } if path == "/tmp/starmap-target.wav"
        )));
    }

    #[test]
    fn non_starmap_instant_audition_does_not_schedule_stable_target_promotion() {
        let mut state =
            crate::native_app::test_support::state::NativeAppStateFixture::default().build();
        let mut context = ui::UiUpdateContext::default();

        state.maybe_schedule_starmap_audition_promotion(
            "/tmp/browser-target.wav",
            "instant_audition",
            &mut context,
        );

        assert!(after_messages(context.into_command()).is_empty());
    }
}
