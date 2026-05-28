use std::path::Path;

pub(in crate::gui_app) struct NormalizedWaveformReload<'a> {
    pub(in crate::gui_app) path: &'a Path,
    pub(in crate::gui_app) playback: Option<WaveformPlaybackResume>,
}

pub(in crate::gui_app) struct WaveformPlaybackResume {
    pub(in crate::gui_app) start_ratio: f32,
    pub(in crate::gui_app) span: Option<(f32, f32)>,
}
