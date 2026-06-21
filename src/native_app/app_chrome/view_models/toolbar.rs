use crate::native_app::app::NativeAppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MainToolbarViewModel {
    pub(in crate::native_app) random_available: bool,
    pub(in crate::native_app) sticky_random_sample_range_playback: bool,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) playing: bool,
    pub(in crate::native_app) beat_guides_enabled: bool,
    pub(in crate::native_app) beat_guide_count: u8,
    pub(in crate::native_app) can_decrement_beat_guide_count: bool,
    pub(in crate::native_app) can_increment_beat_guide_count: bool,
    pub(in crate::native_app) pending_edit_mark_edits: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

impl MainToolbarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        Self {
            random_available: state.random_playback_available(),
            sticky_random_sample_range_playback: state
                .ui
                .chrome
                .sticky_random_sample_range_playback,
            loop_playback: state.audio.loop_playback,
            playing: state.waveform.current.is_playing(),
            beat_guides_enabled: state.ui.chrome.beat_guides_enabled,
            beat_guide_count: state.ui.chrome.beat_guide_count,
            can_decrement_beat_guide_count: state.ui.chrome.beat_guide_count
                > crate::native_app::app::MIN_BEAT_GUIDE_COUNT,
            can_increment_beat_guide_count: state.ui.chrome.beat_guide_count
                < crate::native_app::app::MAX_BEAT_GUIDE_COUNT,
            pending_edit_mark_edits: state
                .waveform
                .current
                .edit_selection()
                .is_some_and(|selection| selection.has_edit_effects()),
            help_tooltips_enabled: state.ui.chrome.help_tooltips_enabled,
        }
    }
}
