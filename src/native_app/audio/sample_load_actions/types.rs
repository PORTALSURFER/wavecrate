use std::path::Path;

pub(in crate::native_app) struct NormalizedWaveformReload<'a> {
    pub(in crate::native_app) path: &'a Path,
    pub(in crate::native_app) playback: Option<WaveformPlaybackResume>,
}

pub(in crate::native_app) struct WaveformPlaybackResume {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) span: Option<(f32, f32)>,
}
