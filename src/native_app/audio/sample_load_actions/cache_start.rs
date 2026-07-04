use radiant::prelude as ui;
use std::{
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
    waveform::{
        PreviewAuditionClip, WaveformPlaybackReady, decode_wav_preview_clip,
        load_cached_waveform_playback_descriptor_sidecar,
    },
    waveform::{file_backed_wav_playback_descriptor, should_use_file_backed_wav_decode},
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
const PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD: usize = 96;

#[derive(Clone, Copy, Debug)]
pub(super) struct FastAuditionOptions {
    pub(super) origin: &'static str,
    pub(super) record_history: bool,
    pub(super) allow_sidecar_lookup: bool,
    pub(super) queue_preview_decode: bool,
    pub(super) prefer_preview_decode: bool,
}

impl NativeAppState {
    pub(super) fn start_fast_path_audition(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
        options: FastAuditionOptions,
    ) -> InstantAuditionOutcome {
        if self.start_preview_cache_instant_audition(path, context, started_at, options) {
            return InstantAuditionOutcome::Started;
        }
        let persisted = self.start_persisted_cache_instant_audition_with_options(
            path,
            context,
            started_at,
            options.allow_sidecar_lookup,
            options.record_history,
            options.origin,
        );
        if persisted.uses_ready_source() {
            return persisted;
        }
        if options.prefer_preview_decode
            && options.queue_preview_decode
            && preview_audition_can_decode(path)
            && self
                .waveform
                .cache
                .preview_audition_warm_needed(Path::new(path))
        {
            self.queue_preview_audition_decode(path.to_owned(), started_at, context);
            return InstantAuditionOutcome::AudioPending;
        }
        if self.start_file_backed_wav_instant_audition_with_options(
            path,
            context,
            started_at,
            options.record_history,
            options.origin,
        ) {
            return InstantAuditionOutcome::Started;
        }
        if options.queue_preview_decode
            && preview_audition_can_decode(path)
            && self
                .waveform
                .cache
                .preview_audition_warm_needed(Path::new(path))
        {
            self.queue_preview_audition_decode(path.to_owned(), started_at, context);
            return InstantAuditionOutcome::AudioPending;
        }
        InstantAuditionOutcome::Unavailable
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
        } else if !self.loop_playback_for_path_after_policy(path)
            && should_use_file_backed_wav_decode(Path::new(path))
        {
            return InstantAuditionOutcome::Unavailable;
        } else if let Some(descriptor) = allow_sidecar_lookup
            .then(|| load_cached_waveform_playback_descriptor_sidecar(PathBuf::from(path)))
            .flatten()
        {
            self.waveform
                .cache
                .mark_sample_playback_descriptor_ready(descriptor.clone());
            descriptor
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
        let Some(runtime) = self.audio.playback_runtime.as_ref() else {
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
        let Some(runtime) = self.audio.playback_runtime.as_ref() else {
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
        let Some(runtime) = self.audio.playback_runtime.as_ref() else {
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
        if options.origin == "starmap_drag" {
            self.schedule_starmap_audition_promotion(path.clone(), context);
        }
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
            return;
        }
        if self.background.preview_audition_warm_task.active().is_some() {
            return;
        }
        let paths = self.preview_audition_warm_candidates();
        if paths.is_empty() {
            return;
        }
        let started_at = Instant::now();
        context
            .business()
            .background(PREVIEW_AUDITION_WARM_TASK_NAME)
            .latest(&mut self.background.preview_audition_warm_task)
            .run(
                move |worker_context| {
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

    fn preview_audition_warm_candidates(&mut self) -> Vec<String> {
        match self.ui.chrome.sample_browser_display {
            SampleBrowserDisplayMode::Map => self.preview_audition_warm_starmap_candidates(),
            SampleBrowserDisplayMode::List => self.preview_audition_warm_list_candidates(),
        }
    }

    fn preview_audition_warm_starmap_candidates(&mut self) -> Vec<String> {
        let Some(items) = self.library.folder_browser.cached_starmap_projection() else {
            return Vec::new();
        };
        let selected = self.library.folder_browser.selected_file_id().map(str::to_owned);
        let center_x = self.ui.chrome.starmap_viewport.center_x;
        let center_y = self.ui.chrome.starmap_viewport.center_y;
        let mut candidates = Vec::new();
        for item in items.iter() {
            if item.missing || !preview_audition_can_decode(&item.file_id) {
                continue;
            }
            let score = if selected.as_deref() == Some(item.file_id.as_str()) {
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
        candidates
            .into_iter()
            .map(|(_, path)| path)
            .take(PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD)
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .take(PREVIEW_AUDITION_WARM_BATCH)
            .collect()
    }

    fn preview_audition_warm_list_candidates(&mut self) -> Vec<String> {
        let visible_paths: Vec<String> = {
            use crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery;

            let visible = self.library.folder_browser.visible_samples(VisibleSampleQuery {
                tags_by_file: &self.metadata.tags_by_file,
                cached_sample_paths: &self.waveform.cache.cached_sample_paths,
            });
            visible
                .rows
                .iter()
                .filter(|row| !row.missing)
                .map(|row| row.file.id.clone())
                .collect()
        };
        visible_paths
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
        if self.library.folder_browser.selected_file_id() != Some(result.path.as_str()) {
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
        let options = FastAuditionOptions {
            origin: self.runtime_playback_origin_for_path(result.path.as_str()),
            record_history: !self.ui.chrome.starmap_audition_drag.is_some(),
            allow_sidecar_lookup: false,
            queue_preview_decode: false,
            prefer_preview_decode: false,
        };
        self.start_preview_clip_instant_audition(clip, context, started_at, options);
    }

    pub(in crate::native_app) fn finish_preview_audition_warm(
        &mut self,
        completion: ui::TaskCompletion<PreviewAuditionWarmResult>,
        started_at: Instant,
    ) {
        let Some(result) = self
            .background
            .preview_audition_warm_task
            .finish_completion(completion)
        else {
            return;
        };
        for path in &result.attempted_paths {
            self.waveform
                .cache
                .mark_preview_audition_attempted(Path::new(path));
        }
        let clip_count = result.clips.len();
        for clip in result.clips {
            self.waveform.cache.store_preview_audition_clip(clip);
        }
        log_sample_load_timing(
            "browser.sample_load.preview_audition.warm",
            "preview-audition-warm",
            started_at.elapsed(),
            false,
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
