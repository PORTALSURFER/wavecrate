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
        GuiMessage, NativeAppState, PendingPlaybackStart, PreviewAuditionResult,
        PreviewAuditionWarmResult, SampleBrowserDisplayMode, SamplePlaybackHistory,
        SamplePlaybackIntent, SamplePlaybackRequest, SamplePlaybackVisibility, WaveformState,
        emit_gui_action, sample_path_label,
    },
    audio::{
        playback::PlaybackIntent,
        sample_load_actions::{log_sample_load_timing, types::SampleLoadStrategy},
    },
    starmap_audition_telemetry as starmap_telemetry,
    waveform::should_use_file_backed_wav_decode,
    waveform::{
        PreviewAuditionClip, WaveformPlaybackReady, decode_wav_preview_clip,
        instant_waveform_head_preview_from_clip, load_cached_waveform_playback_descriptor_sidecar,
    },
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

#[derive(Clone, Copy)]
struct FullSamplePlaybackOptions {
    start_ratio: f32,
    auditioned_start_ratio: Option<f32>,
    replace_policy: PlaybackRuntimeReplacePolicy,
    origin: &'static str,
    history: SamplePlaybackHistory,
    show_start_marker: bool,
    record_history: bool,
}

const SAMPLE_AUTOPLAY_OUTCOMES: CachedPlaybackOutcomes = CachedPlaybackOutcomes {
    playing: "autoplay_started",
    pending: "autoplay_pending",
    error: "autoplay_error",
};

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
    pub(super) allow_file_backed_source: bool,
    pub(super) replace_policy: PlaybackRuntimeReplacePolicy,
}

impl FastAuditionOptions {
    pub(super) fn starmap_drag() -> Self {
        Self {
            origin: "starmap_drag",
            record_history: false,
            allow_sidecar_lookup: false,
            queue_preview_decode: true,
            prefer_preview_decode: false,
            allow_file_backed_source: true,
            replace_policy: PlaybackRuntimeReplacePolicy::FadeOutPrevious,
        }
    }

    pub(super) fn instant_navigation() -> Self {
        Self {
            origin: "instant_audition",
            record_history: true,
            allow_sidecar_lookup: false,
            queue_preview_decode: true,
            prefer_preview_decode: false,
            allow_file_backed_source: true,
            replace_policy: PlaybackRuntimeReplacePolicy::FadeOutPrevious,
        }
    }

    pub(super) fn preview_decode_completion(origin: &'static str, record_history: bool) -> Self {
        Self {
            origin,
            record_history,
            allow_sidecar_lookup: false,
            queue_preview_decode: false,
            prefer_preview_decode: false,
            allow_file_backed_source: true,
            replace_policy: PlaybackRuntimeReplacePolicy::FadeOutPrevious,
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
            FastAuditionProbe::FileBackedWav,
            FastAuditionProbe::PersistedCache,
            FastAuditionProbe::PreviewCache,
            FastAuditionProbe::PreviewDecode,
        ]
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

mod cache_gates;
mod cache_publication;
mod load_planning;
mod playback_ready;
mod telemetry;

use telemetry::{
    record_fast_audition_decision, record_preview_audition_warm_finished,
    record_preview_audition_warm_phase_profile, record_preview_audition_warm_plan,
};
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
#[path = "cache_start_tests.rs"]
mod tests;
