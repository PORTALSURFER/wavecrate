use crate::native_app::app::NativeAppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MainToolbarViewModel {
    pub(in crate::native_app) random_available: bool,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) playing: bool,
}

impl MainToolbarViewModel {
    pub(in crate::native_app) fn from_app_state(state: &NativeAppState) -> Self {
        Self {
            random_available: state.random_playback_available(),
            loop_playback: state.loop_playback,
            playing: state.waveform.is_playing(),
        }
    }
}
