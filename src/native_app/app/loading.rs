use radiant::prelude as ui;

use crate::native_app::waveform::{WaveformPlaybackReady, WaveformState};

pub(in crate::native_app) type SampleLoadTaskCompletion<T> =
    ui::KeyedTaskCompletion<ui::ResourceKey, T>;

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SampleLoadResult {
    pub(in crate::native_app) path: String,
    pub(in crate::native_app) result: Result<WaveformState, String>,
    pub(in crate::native_app) autoplay: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SamplePlaybackReady {
    pub(in crate::native_app) path: String,
    pub(in crate::native_app) audio: WaveformPlaybackReady,
    pub(in crate::native_app) autoplay: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum PendingSamplePlayback {
    RandomAudition { start_unit: f32, length_unit: f32 },
    ResumeNormalized { start: f32, end: f32 },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct SampleSelectionLoadState {
    pub(in crate::native_app) selected_path: Option<String>,
    pub(in crate::native_app) audition: AuditionLoadState,
    pub(in crate::native_app) waveform: WaveformLoadStage,
    pub(in crate::native_app) cache: CacheLoadState,
}

impl SampleSelectionLoadState {
    pub(in crate::native_app) fn start_uncached(&mut self, path: &str) {
        self.selected_path = Some(path.to_owned());
        self.audition = AuditionLoadState::Pending;
        self.waveform = WaveformLoadStage::Loading;
        self.cache = CacheLoadState::Cold;
    }

    pub(in crate::native_app) fn start_cached(&mut self, path: &str) {
        self.selected_path = Some(path.to_owned());
        self.audition = AuditionLoadState::Ready;
        self.waveform = WaveformLoadStage::Ready;
        self.cache = CacheLoadState::Available;
    }

    pub(in crate::native_app) fn playback_ready(&mut self, path: &str) {
        if self.selected_path.as_deref() == Some(path) {
            self.audition = AuditionLoadState::Ready;
        }
    }

    pub(in crate::native_app) fn waveform_ready(&mut self, path: &str) {
        if self.selected_path.as_deref() == Some(path) {
            self.waveform = WaveformLoadStage::Ready;
            self.cache = CacheLoadState::Available;
        }
    }

    pub(in crate::native_app) fn failed(&mut self, path: &str, error: String) {
        if self.selected_path.as_deref() == Some(path) {
            self.audition = AuditionLoadState::Failed(error.clone());
            self.waveform = WaveformLoadStage::Failed(error);
        }
    }

    pub(in crate::native_app) fn cancel(&mut self) {
        self.audition = AuditionLoadState::Cancelled;
        self.waveform = WaveformLoadStage::Idle;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum AuditionLoadState {
    #[default]
    Idle,
    Pending,
    Ready,
    Failed(String),
    Cancelled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformLoadStage {
    #[default]
    Idle,
    Loading,
    Ready,
    Failed(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum CacheLoadState {
    #[default]
    Unknown,
    Cold,
    Available,
}

impl PartialEq for SampleLoadResult {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.result.as_ref().err() == other.result.as_ref().err()
    }
}
