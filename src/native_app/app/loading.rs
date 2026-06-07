use crate::native_app::waveform::{WaveformPlaybackReady, WaveformState};

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
pub(in crate::native_app) struct PendingPlaybackStart {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) loop_offset_ratio: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum PendingSamplePlayback {
    RandomAudition { unit: f32 },
}

impl PartialEq for SampleLoadResult {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.result.as_ref().err() == other.result.as_ref().err()
    }
}
