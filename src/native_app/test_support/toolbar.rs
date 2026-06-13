use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::toolbar as chrome_toolbar;
use crate::native_app::app_chrome::view_models::toolbar::MainToolbarViewModel;
use radiant::prelude as ui;

pub(in crate::native_app) const TOOLBAR_FOCUS_LOADED_ID: u64 =
    chrome_toolbar::TOOLBAR_FOCUS_LOADED_ID;
pub(in crate::native_app) const TOOLBAR_RANDOM_ID: u64 = chrome_toolbar::TOOLBAR_RANDOM_ID;
pub(in crate::native_app) const TOOLBAR_STOP_ID: u64 = chrome_toolbar::TOOLBAR_STOP_ID;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum ToolbarIcon {
    FocusLoaded,
    Loop,
    Random,
    Play,
    Stop,
}

impl ToolbarIcon {
    fn chrome(self) -> chrome_toolbar::ToolbarIcon {
        match self {
            Self::FocusLoaded => chrome_toolbar::ToolbarIcon::FocusLoaded,
            Self::Loop => chrome_toolbar::ToolbarIcon::Loop,
            Self::Random => chrome_toolbar::ToolbarIcon::Random,
            Self::Play => chrome_toolbar::ToolbarIcon::Play,
            Self::Stop => chrome_toolbar::ToolbarIcon::Stop,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MainToolbarProjection {
    pub(in crate::native_app) random_available: bool,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) playing: bool,
}

pub(in crate::native_app) fn main_toolbar(state: &NativeAppState) -> ui::View<GuiMessage> {
    chrome_toolbar::main_toolbar(MainToolbarViewModel::from_app_state(state))
}

pub(in crate::native_app) fn main_toolbar_projection(
    state: &NativeAppState,
) -> MainToolbarProjection {
    let model = MainToolbarViewModel::from_app_state(state);
    MainToolbarProjection {
        random_available: model.random_available,
        loop_playback: model.loop_playback,
        playing: model.playing,
    }
}

pub(in crate::native_app) fn toolbar_icon_button(
    id: u64,
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::View<GuiMessage> {
    chrome_toolbar::toolbar_icon_button(id, icon.chrome(), enabled, active)
}

pub(in crate::native_app) fn toolbar_icon_color(enabled: bool, active: bool) -> ui::Rgba8 {
    chrome_toolbar::toolbar_icon_color(enabled, active)
}

pub(in crate::native_app) fn toolbar_icon_glyph(
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::SvgIcon {
    chrome_toolbar::toolbar_icon_glyph(icon.chrome(), enabled, active)
}
