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
        emit_gui_action, sample_path_label, EarlySamplePlaybackKind, GuiMessage, NativeAppState,
        PendingPlaybackStart, PendingRuntimePlaybackStart, PreviewAuditionResult,
        PreviewAuditionWarmResult, SampleBrowserDisplayMode, SamplePlaybackIntent,
        SamplePlaybackRequest, SamplePlaybackVisibility, WaveformState,
    },
    audio::{
        playback::PlaybackIntent,
        sample_load_actions::{log_sample_load_timing, types::SampleLoadStrategy},
    },
    starmap_audition_telemetry as starmap_telemetry,
    waveform::{
        decode_wav_preview_clip, load_cached_waveform_playback_descriptor_sidecar,
        PreviewAuditionClip, WaveformPlaybackReady,
    },
    waveform::{file_backed_wav_playback_descriptor, should_use_file_backed_wav_decode},
};
use wavecrate::audio::{
    PlaybackRuntimeGainNormalization, PlaybackRuntimeMode, PlaybackRuntimeReplacePolicy,
    PlaybackRuntimeRequest, PlaybackRuntimeSource, PlaybackRuntimeStreamPolicy,
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
const PREVIEW_AUDITION_WARM_PHASE_PROFILE_THRESHOLD: Duration = Duration::from_millis(8);

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

impl FastAuditionProbe {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreviewCache => "preview_cache",
            Self::PersistedCache => "persisted_cache",
            Self::FileBackedWav => "file_backed_wav",
            Self::PreviewDecode => "preview_decode",
        }
    }
}

#[derive(Debug, Default)]
struct PreviewAuditionWarmPlan {
    paths: Vec<String>,
    starmap_signature: Option<u64>,
    list_signature: Option<u64>,
    inspected_count: usize,
    candidate_count: usize,
    eligible_count: usize,
    starmap_cell_count: usize,
    starmap_visited_cell_count: usize,
    starmap_remaining_budget: Option<usize>,
    list_remaining_budget: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
struct PreviewAuditionWarmPhaseSummary {
    scheduled: usize,
    inspected: usize,
    candidates: usize,
    eligible: usize,
    starmap_cells: usize,
    starmap_visited_cells: usize,
    starmap_remaining_budget: usize,
    list_remaining_budget: usize,
}

impl PreviewAuditionWarmPhaseSummary {
    fn from_plan(plan: &PreviewAuditionWarmPlan) -> Self {
        Self {
            scheduled: plan.paths.len(),
            inspected: plan.inspected_count,
            candidates: plan.candidate_count,
            eligible: plan.eligible_count,
            starmap_cells: plan.starmap_cell_count,
            starmap_visited_cells: plan.starmap_visited_cell_count,
            starmap_remaining_budget: plan.starmap_remaining_budget.unwrap_or_default(),
            list_remaining_budget: plan.list_remaining_budget.unwrap_or_default(),
        }
    }
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

fn record_fast_audition_decision(
    path: &str,
    options: FastAuditionOptions,
    probe: Option<FastAuditionProbe>,
    outcome: InstantAuditionOutcome,
    started_at: Option<Instant>,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "fast_audition",
        event = "fast_audition.decision",
        path,
        origin = options.origin,
        probe = probe.map(FastAuditionProbe::as_str).unwrap_or("none"),
        outcome = outcome.as_str(),
        record_history = options.record_history,
        allow_sidecar_lookup = options.allow_sidecar_lookup,
        queue_preview_decode = options.queue_preview_decode,
        prefer_preview_decode = options.prefer_preview_decode,
        allow_file_backed_probe = options.allow_file_backed_probe,
        replace_policy = ?options.replace_policy,
        elapsed_ms = starmap_telemetry::elapsed_since(started_at)
            .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
            .unwrap_or(0.0),
        "Fast audition decision"
    );
}

fn sample_browser_display_mode_str(mode: SampleBrowserDisplayMode) -> &'static str {
    match mode {
        SampleBrowserDisplayMode::List => "list",
        SampleBrowserDisplayMode::Map => "starmap",
    }
}

fn fast_audition_session_intent(options: FastAuditionOptions) -> SamplePlaybackIntent {
    fast_audition_session_intent_for_origin(options.origin)
}

fn fast_audition_session_intent_for_origin(origin: &'static str) -> SamplePlaybackIntent {
    match origin {
        "starmap_drag" => SamplePlaybackIntent::StarmapDrag,
        _ => SamplePlaybackIntent::TransientNavigation,
    }
}

fn fast_audition_session_visibility() -> SamplePlaybackVisibility {
    SamplePlaybackVisibility::Transient
}

fn record_preview_audition_warm_plan(
    display_mode: SampleBrowserDisplayMode,
    outcome: &'static str,
    reason: Option<&'static str>,
    plan: Option<&PreviewAuditionWarmPlan>,
    elapsed: Option<Duration>,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "preview_audition_warm",
        event = "preview_audition.warm_plan",
        display_mode = sample_browser_display_mode_str(display_mode),
        outcome,
        reason = reason.unwrap_or(""),
        scheduled = plan.map(|plan| plan.paths.len()).unwrap_or(0),
        inspected = plan.map(|plan| plan.inspected_count).unwrap_or(0),
        candidates = plan.map(|plan| plan.candidate_count).unwrap_or(0),
        eligible = plan.map(|plan| plan.eligible_count).unwrap_or(0),
        starmap_cells = plan.map(|plan| plan.starmap_cell_count).unwrap_or(0),
        starmap_visited_cells = plan
            .map(|plan| plan.starmap_visited_cell_count)
            .unwrap_or(0),
        starmap_signature = plan
            .and_then(|plan| plan.starmap_signature)
            .unwrap_or_default(),
        list_signature = plan
            .and_then(|plan| plan.list_signature)
            .unwrap_or_default(),
        starmap_remaining_budget = plan
            .and_then(|plan| plan.starmap_remaining_budget)
            .unwrap_or_default(),
        list_remaining_budget = plan
            .and_then(|plan| plan.list_remaining_budget)
            .unwrap_or_default(),
        elapsed_ms = elapsed
            .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
            .unwrap_or(0.0),
        "Preview audition warm plan"
    );
}

fn record_preview_audition_warm_finished(
    scheduled: usize,
    attempted: usize,
    decoded: usize,
    errors: usize,
    worker_elapsed: Duration,
    commit_elapsed: Duration,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    let outcome = if errors > 0 {
        "errors"
    } else if decoded == 0 {
        "empty"
    } else {
        "decoded"
    };
    tracing::info!(
        target: "perf::audio_start",
        module = "preview_audition_warm",
        event = "preview_audition.warm_finished",
        outcome,
        scheduled,
        attempted,
        decoded,
        errors,
        worker_elapsed_ms = worker_elapsed.as_secs_f64() * 1000.0,
        commit_elapsed_ms = commit_elapsed.as_secs_f64() * 1000.0,
        "Preview audition warm finished"
    );
}

fn record_preview_audition_warm_phase_profile(
    display_mode: SampleBrowserDisplayMode,
    outcome: &'static str,
    reason: Option<&'static str>,
    summary: PreviewAuditionWarmPhaseSummary,
    total_elapsed: Duration,
    plan_elapsed: Duration,
    reservation_elapsed: Duration,
    task_schedule_elapsed: Duration,
) {
    let telemetry_enabled = starmap_telemetry::enabled();
    let slow = total_elapsed >= PREVIEW_AUDITION_WARM_PHASE_PROFILE_THRESHOLD;
    if !slow && !telemetry_enabled {
        return;
    }
    if slow {
        tracing::warn!(
            target: "wavecrate::debug::ui_frame",
            module = "preview_audition_warm",
            event = "preview_audition.warm_phase_profile",
            display_mode = sample_browser_display_mode_str(display_mode),
            outcome,
            reason = reason.unwrap_or(""),
            scheduled = summary.scheduled,
            inspected = summary.inspected,
            candidates = summary.candidates,
            eligible = summary.eligible,
            starmap_cells = summary.starmap_cells,
            starmap_visited_cells = summary.starmap_visited_cells,
            starmap_remaining_budget = summary.starmap_remaining_budget,
            list_remaining_budget = summary.list_remaining_budget,
            total_elapsed_ms = total_elapsed.as_secs_f64() * 1000.0,
            plan_elapsed_ms = plan_elapsed.as_secs_f64() * 1000.0,
            reservation_elapsed_ms = reservation_elapsed.as_secs_f64() * 1000.0,
            task_schedule_elapsed_ms = task_schedule_elapsed.as_secs_f64() * 1000.0,
            "Slow preview audition warm phase"
        );
    } else {
        tracing::info!(
            target: "perf::audio_start",
            module = "preview_audition_warm",
            event = "preview_audition.warm_phase_profile",
            display_mode = sample_browser_display_mode_str(display_mode),
            outcome,
            reason = reason.unwrap_or(""),
            scheduled = summary.scheduled,
            inspected = summary.inspected,
            candidates = summary.candidates,
            eligible = summary.eligible,
            starmap_cells = summary.starmap_cells,
            starmap_visited_cells = summary.starmap_visited_cells,
            starmap_remaining_budget = summary.starmap_remaining_budget,
            list_remaining_budget = summary.list_remaining_budget,
            total_elapsed_ms = total_elapsed.as_secs_f64() * 1000.0,
            plan_elapsed_ms = plan_elapsed.as_secs_f64() * 1000.0,
            reservation_elapsed_ms = reservation_elapsed.as_secs_f64() * 1000.0,
            task_schedule_elapsed_ms = task_schedule_elapsed.as_secs_f64() * 1000.0,
            "Preview audition warm phase profile"
        );
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
        self.audio.early_sample_playback_path = Some(path.to_owned());
        self.audio.early_sample_playback_kind = Some(EarlySamplePlaybackKind::FullSample);
        let visibility = fast_audition_session_visibility();
        self.audio.current_playback_span =
            visibility.updates_waveform_playhead().then_some((0.0, 1.0));
        let session_request = SamplePlaybackRequest {
            path: path.to_owned(),
            span: (0.0, 1.0),
            intent: fast_audition_session_intent_for_origin(origin),
            visibility,
            stream_policy: PlaybackRuntimeStreamPolicy::transient_navigation(),
            show_start_marker: visibility.updates_waveform_playhead(),
        };
        let session_generation = self.audio.start_sample_playback_session(
            session_request.clone(),
            request_id,
            "interleaved_f32_file",
        );
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            session_generation,
            session_request.path,
            session_request.span,
            session_request.show_start_marker,
            visibility,
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
        self.audio.early_sample_playback_path = Some(path.to_owned());
        self.audio.early_sample_playback_kind = Some(EarlySamplePlaybackKind::FullSample);
        let visibility = SamplePlaybackVisibility::Transient;
        self.audio.current_playback_span = None;
        let session_request = SamplePlaybackRequest {
            path: path.to_owned(),
            span: (0.0, 1.0),
            intent: SamplePlaybackIntent::TransientNavigation,
            visibility,
            stream_policy: PlaybackRuntimeStreamPolicy::transient_navigation(),
            show_start_marker: false,
        };
        let session_generation = self.audio.start_sample_playback_session(
            session_request.clone(),
            request_id,
            "audio_file",
        );
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            session_generation,
            session_request.path,
            session_request.span,
            session_request.show_start_marker,
            visibility,
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
        self.audio.early_sample_playback_path = Some(path.clone());
        self.audio.early_sample_playback_kind = Some(EarlySamplePlaybackKind::PreviewSlice);
        self.audio.current_playback_span = None;
        let visibility = fast_audition_session_visibility();
        let session_request = SamplePlaybackRequest {
            path: path.clone(),
            span: (0.0, 1.0),
            intent: fast_audition_session_intent(options),
            visibility,
            stream_policy: PlaybackRuntimeStreamPolicy::transient_navigation(),
            show_start_marker: false,
        };
        let session_generation = self.audio.start_sample_playback_session(
            session_request.clone(),
            request_id,
            "preview_samples",
        );
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            session_generation,
            session_request.path,
            session_request.span,
            session_request.show_start_marker,
            visibility,
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
        let phase_started_at = Instant::now();
        let display_mode = self.ui.chrome.sample_browser_display;
        if self.preview_audition_warm_should_yield() {
            self.background.preview_audition_warm_task.cancel();
            self.waveform.cache.cancel_preview_audition_warm_schedule();
            let reason = self.preview_audition_warm_yield_reason();
            record_preview_audition_warm_plan(display_mode, "yield", Some(reason), None, None);
            record_preview_audition_warm_phase_profile(
                display_mode,
                "yield",
                Some(reason),
                PreviewAuditionWarmPhaseSummary::default(),
                phase_started_at.elapsed(),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
            );
            return;
        }
        if self
            .background
            .preview_audition_warm_task
            .active()
            .is_some()
        {
            record_preview_audition_warm_phase_profile(
                display_mode,
                "active",
                None,
                PreviewAuditionWarmPhaseSummary::default(),
                phase_started_at.elapsed(),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
            );
            return;
        }
        let plan_started_at = Instant::now();
        let plan = self.preview_audition_warm_candidates();
        let plan_elapsed = plan_started_at.elapsed();
        if plan.paths.is_empty() {
            record_preview_audition_warm_plan(
                display_mode,
                "empty",
                None,
                Some(&plan),
                Some(plan_elapsed),
            );
            record_preview_audition_warm_phase_profile(
                display_mode,
                "empty",
                None,
                PreviewAuditionWarmPhaseSummary::from_plan(&plan),
                phase_started_at.elapsed(),
                plan_elapsed,
                Duration::ZERO,
                Duration::ZERO,
            );
            return;
        }
        record_preview_audition_warm_plan(
            display_mode,
            "scheduled",
            None,
            Some(&plan),
            Some(plan_elapsed),
        );
        let summary = PreviewAuditionWarmPhaseSummary::from_plan(&plan);
        let reservation_started_at = Instant::now();
        let paths = plan.paths;
        self.waveform
            .cache
            .mark_preview_audition_warm_scheduled(&paths);
        if let Some(signature) = plan.starmap_signature {
            self.waveform
                .cache
                .reserve_starmap_preview_warm_batch(signature, paths.len());
        }
        if let Some(signature) = plan.list_signature {
            self.waveform
                .cache
                .reserve_list_preview_warm_batch(signature, paths.len());
        }
        let reservation_elapsed = reservation_started_at.elapsed();
        let started_at = Instant::now();
        let task_schedule_started_at = Instant::now();
        context
            .business()
            .background(PREVIEW_AUDITION_WARM_TASK_NAME)
            .latest(&mut self.background.preview_audition_warm_task)
            .run(
                move |worker_context| {
                    let scheduled_paths = paths.clone();
                    let mut attempted_paths = Vec::new();
                    let mut failed_paths = Vec::new();
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
                            Err(_) => {
                                errors += 1;
                                failed_paths.push(path);
                            }
                        }
                    }
                    PreviewAuditionWarmResult {
                        scheduled_paths,
                        attempted_paths,
                        failed_paths,
                        clips,
                        errors,
                    }
                },
                move |completion| GuiMessage::PreviewAuditionWarmFinished {
                    completion,
                    started_at,
                },
            );
        record_preview_audition_warm_phase_profile(
            display_mode,
            "scheduled",
            None,
            summary,
            phase_started_at.elapsed(),
            plan_elapsed,
            reservation_elapsed,
            task_schedule_started_at.elapsed(),
        );
    }

    fn preview_audition_warm_should_yield(&self) -> bool {
        self.ui.chrome.starmap_audition_drag.is_some()
            || self.sample_cache_warm_should_pause_active()
            || self.playback_visual_activity_active()
    }

    fn preview_audition_warm_yield_reason(&self) -> &'static str {
        if self.ui.chrome.starmap_audition_drag.is_some() {
            "starmap_drag"
        } else if self.sample_cache_warm_should_pause_active() {
            "sample_load_or_normalization"
        } else if self.playback_visual_activity_active() {
            "playback_active"
        } else {
            "unknown"
        }
    }

    fn preview_audition_warm_candidates(&mut self) -> PreviewAuditionWarmPlan {
        match self.ui.chrome.sample_browser_display {
            SampleBrowserDisplayMode::Map => self.preview_audition_warm_starmap_candidates(),
            SampleBrowserDisplayMode::List => self.preview_audition_warm_list_candidates(),
        }
    }

    fn preview_audition_warm_starmap_candidates(&mut self) -> PreviewAuditionWarmPlan {
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
            self.library.folder_browser.cached_starmap_projection_len(),
            center_x,
            center_y,
            zoom,
        );
        let mut remaining_budget = self
            .waveform
            .cache
            .remaining_starmap_preview_warm_budget(signature, PREVIEW_AUDITION_STARMAP_VIEW_BUDGET);
        if remaining_budget == 0 {
            return PreviewAuditionWarmPlan {
                paths: Vec::new(),
                starmap_signature: Some(signature),
                inspected_count: 0,
                candidate_count: 0,
                eligible_count: 0,
                starmap_remaining_budget: Some(0),
                ..PreviewAuditionWarmPlan::default()
            };
        }
        let Some(candidates) = self
            .library
            .folder_browser
            .cached_starmap_preview_warm_candidates(
                center_x,
                center_y,
                zoom,
                PREVIEW_AUDITION_STARMAP_VIEWPORT_PAD,
                selected.as_deref(),
                PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD,
            )
        else {
            return PreviewAuditionWarmPlan::default();
        };
        let candidate_count = candidates.indices.len();
        let eligible_paths = candidates
            .indices
            .iter()
            .map(|&index| candidates.items[index].file_id.clone())
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .take(PREVIEW_AUDITION_STARMAP_NEIGHBORHOOD)
            .collect::<Vec<_>>();
        let eligible_count = eligible_paths.len();
        if eligible_count == 0 && remaining_budget > 0 {
            self.waveform
                .cache
                .reserve_starmap_preview_warm_budget(signature, remaining_budget);
            remaining_budget = 0;
        }
        let paths = eligible_paths
            .into_iter()
            .take(PREVIEW_AUDITION_WARM_BATCH.min(remaining_budget))
            .collect();
        PreviewAuditionWarmPlan {
            paths,
            starmap_signature: Some(signature),
            inspected_count: candidates.inspected_count,
            candidate_count,
            eligible_count,
            starmap_cell_count: candidates.cell_count,
            starmap_visited_cell_count: candidates.visited_cell_count,
            starmap_remaining_budget: Some(remaining_budget),
            ..PreviewAuditionWarmPlan::default()
        }
    }

    fn preview_audition_warm_list_candidates(&mut self) -> PreviewAuditionWarmPlan {
        let ordered_paths: Vec<String> = {
            let Some(visible_paths) = self
                .library
                .folder_browser
                .prepared_visible_sample_file_ids_matching_tags(
                    &self.metadata.tags_by_file,
                    PREVIEW_AUDITION_LIST_VIEW_BUDGET,
                )
            else {
                return PreviewAuditionWarmPlan::default();
            };
            Self::preview_audition_list_warm_ordered_paths(
                &visible_paths,
                self.library.folder_browser.selected_file_id(),
                PREVIEW_AUDITION_LIST_VIEW_BUDGET,
            )
        };
        let signature = list_preview_warm_view_signature(
            self.library.folder_browser.selected_source_id(),
            &ordered_paths,
        );
        let mut remaining_budget = self
            .waveform
            .cache
            .remaining_list_preview_warm_budget(signature, PREVIEW_AUDITION_LIST_VIEW_BUDGET);
        if remaining_budget == 0 {
            return PreviewAuditionWarmPlan {
                paths: Vec::new(),
                list_signature: Some(signature),
                inspected_count: 0,
                candidate_count: 0,
                eligible_count: 0,
                list_remaining_budget: Some(0),
                ..PreviewAuditionWarmPlan::default()
            };
        }
        let inspected_count = ordered_paths.len();
        let candidate_paths = ordered_paths
            .into_iter()
            .filter(|path| preview_audition_can_decode(path))
            .collect::<Vec<_>>();
        let candidate_count = candidate_paths.len();
        let eligible_paths = candidate_paths
            .into_iter()
            .filter(|path| {
                self.waveform
                    .cache
                    .preview_audition_warm_needed(Path::new(path))
            })
            .collect::<Vec<_>>();
        let eligible_count = eligible_paths.len();
        if eligible_count == 0 && remaining_budget > 0 {
            self.waveform
                .cache
                .reserve_list_preview_warm_budget(signature, remaining_budget);
            remaining_budget = 0;
        }
        let paths = eligible_paths
            .into_iter()
            .take(PREVIEW_AUDITION_WARM_BATCH.min(remaining_budget))
            .collect();
        PreviewAuditionWarmPlan {
            paths,
            list_signature: Some(signature),
            inspected_count,
            candidate_count,
            eligible_count,
            list_remaining_budget: Some(remaining_budget),
            ..PreviewAuditionWarmPlan::default()
        }
    }

    fn preview_audition_list_warm_ordered_paths(
        rows: &[String],
        selected_file_id: Option<&str>,
        limit: usize,
    ) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }
        let selected_index =
            selected_file_id.and_then(|selected| rows.iter().position(|row| row == selected));
        let Some(selected_index) = selected_index else {
            return rows.iter().take(limit).cloned().collect();
        };
        let mut ordered = Vec::with_capacity(limit.min(rows.len()));
        for offset in 0..rows.len() {
            if offset == 0 {
                if let Some(row) = rows.get(selected_index) {
                    ordered.push(row.clone());
                }
            } else {
                if let Some(row) = selected_index
                    .checked_add(offset)
                    .and_then(|index| rows.get(index))
                {
                    ordered.push(row.clone());
                }
                if ordered.len() >= limit {
                    break;
                }
                if let Some(row) = selected_index
                    .checked_sub(offset)
                    .and_then(|index| rows.get(index))
                {
                    ordered.push(row.clone());
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
                    .mark_preview_audition_failed(Path::new(result.path.as_str()));
                emit_gui_action(
                    "browser.select_sample",
                    Some("browser"),
                    Some(&sample_path_label(result.path.as_str())),
                    "preview_audition_error",
                    started_at,
                    Some(&error),
                );
                self.advance_starmap_audition_after_preview_decode_failure(
                    result.path.as_str(),
                    context,
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

    fn advance_starmap_audition_after_preview_decode_failure(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self
            .ui
            .chrome
            .starmap_audition_queue
            .active_file_id
            .as_deref()
            != Some(path)
        {
            return;
        }
        self.ui.chrome.starmap_audition_queue.active_file_id = None;
        context.request_paint_only();
        self.start_next_starmap_audition_hit(context);
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
            &result.failed_paths,
        );
        let scheduled_count = result.scheduled_paths.len();
        let attempted_count = result.attempted_paths.len();
        let clip_count = result.clips.len();
        let error_count = result.errors;
        for clip in result.clips {
            self.waveform.cache.store_preview_audition_clip(clip);
        }
        let worker_elapsed = started_at.elapsed();
        let commit_elapsed = finish_started_at.elapsed();
        record_preview_audition_warm_finished(
            scheduled_count,
            attempted_count,
            clip_count,
            error_count,
            worker_elapsed,
            commit_elapsed,
        );
        log_sample_load_timing(
            "browser.sample_load.preview_audition.warm_commit",
            "preview-audition-warm",
            commit_elapsed,
            false,
        );
        tracing::debug!(
            target: "wavecrate::debug::sample_load",
            event = "browser.sample_load.preview_audition.warm_finished",
            scheduled = scheduled_count,
            attempted = attempted_count,
            decoded = clip_count,
            errors = error_count,
            worker_elapsed_ms = worker_elapsed.as_secs_f64() * 1000.0,
            commit_elapsed_ms = commit_elapsed.as_secs_f64() * 1000.0,
            "Preview audition warm finished"
        );
        if error_count > 0 {
            tracing::debug!(
                attempted = attempted_count,
                decoded = clip_count,
                errors = error_count,
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
        self.audio.early_sample_playback_path = Some(path.clone());
        self.audio.early_sample_playback_kind = Some(EarlySamplePlaybackKind::FullSample);
        self.audio.current_playback_span = Some((0.0, 1.0));
        let origin = self.runtime_playback_origin_for_path(path.as_str());
        let session_request = SamplePlaybackRequest {
            path,
            span: (0.0, 1.0),
            intent: SamplePlaybackIntent::ExplicitPlayback,
            visibility: SamplePlaybackVisibility::Waveform,
            stream_policy: PlaybackRuntimeStreamPolicy::full(),
            show_start_marker: true,
        };
        let session_generation = self.audio.start_sample_playback_session(
            session_request.clone(),
            request_id,
            "decoded_samples",
        );
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            session_generation,
            session_request.path,
            session_request.span,
            session_request.show_start_marker,
            session_request.visibility,
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
        if !self.waveform.current.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        let current_path = self.waveform.current.path().display().to_string();
        let start_ratio = start_ratio.clamp(0.0, 0.999);
        if self.audio.early_sample_playback_path.as_deref() == Some(current_path.as_str()) {
            self.audio.early_sample_playback_path = None;
            self.audio.early_sample_playback_kind = None;
        }
        self.prepare_playback_mode_for_loaded_sample();
        if self.audio.playback_runtime.is_none() {
            self.audio.pending_playback_start = Some(PendingPlaybackStart::record(
                PlaybackIntent::new(start_ratio, 1.0),
            ));
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
            replace_policy,
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
        self.waveform.current.start_playback(start_ratio);
        self.audio.current_playback_span = Some((0.0, 1.0));
        let origin = self.runtime_playback_origin_for_path(current_path.as_str());
        let source_kind = self.current_waveform_runtime_source_kind();
        let session_request = SamplePlaybackRequest {
            path: current_path.clone(),
            span: (0.0, 1.0),
            intent: SamplePlaybackIntent::ExplicitPlayback,
            visibility: SamplePlaybackVisibility::Waveform,
            stream_policy: PlaybackRuntimeStreamPolicy::full(),
            show_start_marker: true,
        };
        let session_generation = self.audio.start_sample_playback_session(
            session_request.clone(),
            request_id,
            source_kind,
        );
        self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart::new(
            request_id,
            session_generation,
            session_request.path,
            session_request.span,
            session_request.show_start_marker,
            session_request.visibility,
            origin,
            source_kind,
        ));
        self.record_current_playback_history(0.0, 1.0);
        Ok(())
    }

    pub(in crate::native_app::audio) fn preview_slice_full_sample_handoff_ratio(
        &self,
        path: &str,
    ) -> Option<f32> {
        if self.audio.early_sample_playback_path.as_deref() != Some(path)
            || self.audio.early_sample_playback_kind != Some(EarlySamplePlaybackKind::PreviewSlice)
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

fn list_preview_warm_view_signature(source_id: &str, ordered_paths: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    source_id.hash(&mut hasher);
    ordered_paths.hash(&mut hasher);
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
    use std::{
        collections::HashSet,
        fs,
        path::{Path, PathBuf},
        sync::Arc,
        time::SystemTime,
    };

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
        crate::native_app::test_support::sample_browser::complete_starmap_layout_for_selected_source(
            &mut state,
        );
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

    fn unprepared_list_state_with_wav_files(
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

    fn write_sparse_wav_i16(path: &Path, channels: u16, frames: usize) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        for frame in 0..frames {
            for channel in 0..channels {
                let sample = ((frame + usize::from(channel)) % 256) as i16;
                writer.write_sample(sample).expect("write sample");
            }
        }
        writer.finalize().expect("finalize wav");
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
    fn starmap_drag_long_wav_queues_preview_head_decode() {
        let source_root = tempfile::tempdir().expect("source root");
        let sample = source_root.path().join("long.wav");
        write_sparse_wav_i16(&sample, 1, 700);
        let sample_id = sample.display().to_string();
        let mut state = crate::native_app::test_support::state::NativeAppStateFixture::default()
            .with_folder_browser(
                crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
                    wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
                ]),
            )
            .build();
        state.ui.chrome.sample_browser_display = SampleBrowserDisplayMode::Map;
        state.ui.chrome.starmap_audition_drag =
            Some(crate::native_app::app::StarmapAuditionDragState {
                last_hit_file_id: Some(sample_id.clone()),
                last_position: ui::Point::new(0.0, 0.0),
                modifiers: Default::default(),
            });
        state.ui.chrome.starmap_audition_queue.active_file_id = Some(sample_id.clone());
        let mut context = ui::UiUpdateContext::default();

        assert!(
            crate::native_app::waveform::should_use_file_backed_wav_decode(&sample),
            "fixture should exercise the long/file-backed WAV threshold"
        );
        let outcome = state.start_fast_path_audition(
            sample_id.as_str(),
            &mut context,
            Instant::now(),
            FastAuditionOptions::starmap_drag(),
        );

        assert_eq!(
            outcome,
            InstantAuditionOutcome::AudioPending,
            "long WAVs should queue the tiny preview-head decode instead of falling through to full foreground loading"
        );
        assert!(
            state.background.preview_audition_task.active().is_some(),
            "preview-head decode should be tracked as the active cancellable task"
        );
        assert!(
            state.audio.pending_runtime_start.is_none(),
            "the long WAV path should not synchronously probe or submit file-backed playback on the UI path"
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
            assert_eq!(plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
            if warmed.is_empty() {
                assert_eq!(plan.inspected_count, PREVIEW_AUDITION_LIST_VIEW_BUDGET);
                assert_eq!(plan.candidate_count, PREVIEW_AUDITION_LIST_VIEW_BUDGET);
                assert_eq!(plan.eligible_count, PREVIEW_AUDITION_LIST_VIEW_BUDGET);
            }
            warmed.extend(plan.paths.iter().cloned());
            reserve_preview_warm_plan(&mut state, &plan);
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
            exhausted_plan.paths.len(),
            0,
            "list preview warming should stop after the selected-row neighborhood budget"
        );
        assert_eq!(exhausted_plan.inspected_count, 0);
        assert_eq!(exhausted_plan.candidate_count, 0);
        assert_eq!(exhausted_plan.eligible_count, 0);
    }

    #[test]
    fn list_preview_warm_skips_until_visible_window_is_prepared() {
        let (mut state, _paths) = unprepared_list_state_with_wav_files(256, 128);
        let cache_len_before = state
            .library
            .folder_browser
            .selected_audio_projection_cache_len_for_tests();

        let plan = state.preview_audition_warm_list_candidates();

        assert_eq!(
            plan.paths.len(),
            0,
            "preview warming should not build the list projection before the visible list is prepared"
        );
        assert_eq!(
            state
                .library
                .folder_browser
                .selected_audio_projection_cache_len_for_tests(),
            cache_len_before,
            "preview warming must stay opportunistic instead of filling the projection cache on the UI frame"
        );
    }

    #[test]
    fn list_preview_warm_exhausts_sparse_view_after_attempted_candidates() {
        let (mut state, _) = list_state_with_wav_files(PREVIEW_AUDITION_WARM_BATCH / 2, 0);
        let first_plan = state.preview_audition_warm_list_candidates();
        assert!(
            !first_plan.paths.is_empty(),
            "sparse list fixture should still have warmable candidates"
        );
        assert!(
            first_plan.paths.len() < PREVIEW_AUDITION_LIST_VIEW_BUDGET,
            "fixture must leave budget remaining after the first sparse warm"
        );
        reserve_preview_warm_plan(&mut state, &first_plan);
        state.waveform.cache.finish_preview_audition_warm_schedule(
            &first_plan.paths,
            &first_plan.paths,
            &[],
        );

        let exhausted_plan = state.preview_audition_warm_list_candidates();

        assert_eq!(exhausted_plan.paths.len(), 0);
        assert_eq!(
            exhausted_plan.list_remaining_budget,
            Some(0),
            "already-attempted sparse list views should not be re-planned forever"
        );

        let repeated_plan = state.preview_audition_warm_list_candidates();

        assert_eq!(
            repeated_plan.inspected_count, 0,
            "exhausted list warm views should skip candidate inspection"
        );
    }

    #[test]
    fn list_preview_warm_cancel_releases_view_budget_for_retry() {
        let (mut state, _) = list_state_with_wav_files(PREVIEW_AUDITION_LIST_VIEW_BUDGET + 8, 0);
        let first_plan = state.preview_audition_warm_list_candidates();
        assert_eq!(first_plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
        reserve_preview_warm_plan(&mut state, &first_plan);

        state.waveform.cache.cancel_preview_audition_warm_schedule();
        let retry_plan = state.preview_audition_warm_list_candidates();

        assert_eq!(
            retry_plan.list_remaining_budget,
            Some(PREVIEW_AUDITION_LIST_VIEW_BUDGET),
            "cancelled list warm work should not consume the finite viewport budget"
        );
        assert_eq!(
            retry_plan.paths, first_plan.paths,
            "cancelled list warm work should retry the same nearest candidates once idle"
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

    fn reserve_preview_warm_plan(
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
                .reserve_starmap_preview_warm_batch(signature, plan.paths.len());
        }
        if let Some(signature) = plan.list_signature {
            state
                .waveform
                .cache
                .reserve_list_preview_warm_batch(signature, plan.paths.len());
        }
    }

    #[test]
    fn starmap_preview_warm_stops_after_view_budget_until_view_changes() {
        let mut state = starmap_state_with_wav_files(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET + 48);
        let mut warmed = HashSet::new();

        for _ in 0..(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET / PREVIEW_AUDITION_WARM_BATCH) {
            let plan = state.preview_audition_warm_starmap_candidates();
            assert_eq!(plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
            if warmed.is_empty() {
                assert_eq!(
                    plan.starmap_remaining_budget,
                    Some(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET)
                );
                assert!(plan.eligible_count >= PREVIEW_AUDITION_WARM_BATCH);
            }
            warmed.extend(plan.paths.iter().cloned());
            reserve_preview_warm_plan(&mut state, &plan);
        }
        assert_eq!(warmed.len(), PREVIEW_AUDITION_STARMAP_VIEW_BUDGET);

        let exhausted_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(
            exhausted_plan.paths.len(),
            0,
            "starmap preview warming should not keep crawling a dense unchanged map forever"
        );
        assert_eq!(exhausted_plan.starmap_remaining_budget, Some(0));

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
            reserve_preview_warm_plan(&mut state, &plan);
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
    fn starmap_preview_warm_exhausts_sparse_view_after_attempted_candidates() {
        let mut state = starmap_state_with_wav_files(PREVIEW_AUDITION_WARM_BATCH / 2);
        let first_plan = state.preview_audition_warm_starmap_candidates();
        assert!(
            !first_plan.paths.is_empty(),
            "sparse starmap fixture should still have warmable candidates"
        );
        assert!(
            first_plan.paths.len() < PREVIEW_AUDITION_STARMAP_VIEW_BUDGET,
            "fixture must leave budget remaining after the first sparse warm"
        );
        reserve_preview_warm_plan(&mut state, &first_plan);
        state.waveform.cache.finish_preview_audition_warm_schedule(
            &first_plan.paths,
            &first_plan.paths,
            &[],
        );

        let exhausted_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(exhausted_plan.paths.len(), 0);
        assert_eq!(
            exhausted_plan.starmap_remaining_budget,
            Some(0),
            "already-attempted sparse starmap views should not be re-planned forever"
        );

        let repeated_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(
            repeated_plan.inspected_count, 0,
            "exhausted starmap warm views should skip candidate inspection"
        );
    }

    #[test]
    fn starmap_preview_warm_cancel_releases_view_budget_for_retry() {
        let mut state = starmap_state_with_wav_files(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET + 48);
        let first_plan = state.preview_audition_warm_starmap_candidates();
        assert_eq!(first_plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
        reserve_preview_warm_plan(&mut state, &first_plan);

        state.waveform.cache.cancel_preview_audition_warm_schedule();
        let retry_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(
            retry_plan.starmap_remaining_budget,
            Some(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET),
            "cancelled starmap warm work should not consume the finite viewport budget"
        );
        assert_eq!(
            retry_plan.paths, first_plan.paths,
            "cancelled starmap warm work should retry the same viewport candidates once idle"
        );
    }

    #[test]
    fn starmap_preview_warm_partial_finish_consumes_only_attempted_budget() {
        let mut state = starmap_state_with_wav_files(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET + 48);
        let first_plan = state.preview_audition_warm_starmap_candidates();
        assert_eq!(first_plan.paths.len(), PREVIEW_AUDITION_WARM_BATCH);
        let attempted = first_plan.paths[0].clone();
        reserve_preview_warm_plan(&mut state, &first_plan);
        state.waveform.cache.finish_preview_audition_warm_schedule(
            &first_plan.paths,
            std::slice::from_ref(&attempted),
            &[],
        );

        let next_plan = state.preview_audition_warm_starmap_candidates();

        assert_eq!(
            next_plan.starmap_remaining_budget,
            Some(PREVIEW_AUDITION_STARMAP_VIEW_BUDGET - 1),
            "unattempted starmap warm tails should not be charged against the viewport budget"
        );
        assert!(
            next_plan.paths.iter().any(|path| path != &attempted),
            "partial completion should leave later candidates available for warming"
        );
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
