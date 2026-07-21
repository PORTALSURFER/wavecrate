use crate::native_app::app::NativeAppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MainToolbarViewModel {
    pub(in crate::native_app) random_available: bool,
    pub(in crate::native_app) similar_sections_available: bool,
    pub(in crate::native_app) similar_sections_enabled: bool,
    pub(in crate::native_app) sticky_random_sample_range_playback: bool,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) playing: bool,
    pub(in crate::native_app) zero_crossing_snap_enabled: bool,
    pub(in crate::native_app) bpm_snap_enabled: bool,
    pub(in crate::native_app) beat_guides_enabled: bool,
    pub(in crate::native_app) metronome_enabled: bool,
    pub(in crate::native_app) beat_guide_count: u8,
    pub(in crate::native_app) pending_edit_mark_edits: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

impl MainToolbarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        Self {
            random_available: state.random_playback_available(),
            similar_sections_available: state
                .waveform
                .current
                .play_selection()
                .is_some_and(|selection| selection.width() > 0.0),
            similar_sections_enabled: state.waveform.current.similar_sections_enabled(),
            sticky_random_sample_range_playback: state
                .ui
                .chrome
                .sticky_random_sample_range_playback,
            loop_playback: state.audio.loop_playback,
            playing: state.waveform.current.is_playing(),
            zero_crossing_snap_enabled: state.waveform.current.zero_crossing_snap_enabled(),
            bpm_snap_enabled: state.ui.chrome.bpm_snap_enabled,
            beat_guides_enabled: state.ui.chrome.beat_guides_enabled,
            metronome_enabled: state.audio.metronome_enabled,
            beat_guide_count: state.ui.chrome.beat_guide_count,
            pending_edit_mark_edits: state
                .waveform
                .current
                .edit_selection()
                .is_some_and(|selection| selection.has_edit_effects()),
            help_tooltips_enabled: state.ui.chrome.help_tooltips_enabled,
        }
    }
}
