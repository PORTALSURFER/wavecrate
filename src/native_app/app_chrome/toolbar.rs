use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::ui::ids as widget_ids;

const TOOLBAR_ICON_ACTIVE_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 160, 82, 255);
const TOOLBAR_ICON_ENABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(238, 238, 238, 255);
const TOOLBAR_ICON_DISABLED_COLOR: ui::Rgba8 = ui::Rgba8::new(145, 145, 145, 255);

pub(in crate::native_app) const TOOLBAR_FOCUS_LOADED_ID: u64 = widget_ids::TOOLBAR_FOCUS_LOADED_ID;
const TOOLBAR_LOOP_ID: u64 = widget_ids::TOOLBAR_LOOP_ID;
const TOOLBAR_PLAY_ID: u64 = widget_ids::TOOLBAR_PLAY_ID;
pub(in crate::native_app) const TOOLBAR_STOP_ID: u64 = widget_ids::TOOLBAR_STOP_ID;
pub(in crate::native_app) const TOOLBAR_RANDOM_ID: u64 = widget_ids::TOOLBAR_RANDOM_ID;

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

pub(in crate::native_app) fn main_toolbar(model: MainToolbarViewModel) -> ui::View<GuiMessage> {
    ui::toolbar_from_parts(
        ui::ToolbarParts::new([
            toolbar_icon_button(
                TOOLBAR_FOCUS_LOADED_ID,
                ToolbarIcon::FocusLoaded,
                true,
                false,
            ),
            toolbar_icon_button(
                TOOLBAR_LOOP_ID,
                ToolbarIcon::Loop,
                true,
                model.loop_playback,
            ),
            toolbar_icon_button(
                TOOLBAR_RANDOM_ID,
                ToolbarIcon::Random,
                model.random_available,
                false,
            ),
            toolbar_icon_button(TOOLBAR_PLAY_ID, ToolbarIcon::Play, true, model.playing),
            toolbar_icon_button(TOOLBAR_STOP_ID, ToolbarIcon::Stop, true, false),
        ])
        .align_end(),
    )
}

pub(in crate::native_app) fn toolbar_icon_button(
    id: u64,
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::View<GuiMessage> {
    ui::icon_button(toolbar_icon_glyph(icon, enabled, active))
        .enabled(enabled)
        .active(active)
        .message(toolbar_button_message(icon))
        .id(id)
        .size(28.0, 24.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum ToolbarIcon {
    FocusLoaded,
    Loop,
    Random,
    Play,
    Stop,
}

impl ToolbarIcon {
    fn cache(self) -> &'static ui::SvgIconTintCache {
        match self {
            Self::FocusLoaded => &FOCUS_LOADED_ICON,
            Self::Loop => &LOOP_ICON,
            Self::Random => &RANDOM_ICON,
            Self::Play => &PLAY_ICON,
            Self::Stop => &STOP_ICON,
        }
    }
}

pub(in crate::native_app) fn toolbar_icon_color(enabled: bool, active: bool) -> ui::Rgba8 {
    if !enabled {
        TOOLBAR_ICON_DISABLED_COLOR
    } else if active {
        TOOLBAR_ICON_ACTIVE_COLOR
    } else {
        TOOLBAR_ICON_ENABLED_COLOR
    }
}

pub(in crate::native_app) fn toolbar_icon_glyph(
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::SvgIcon {
    icon.cache().icon(toolbar_icon_color(enabled, active))
}

fn toolbar_button_message(icon: ToolbarIcon) -> GuiMessage {
    match icon {
        ToolbarIcon::FocusLoaded => GuiMessage::FocusLoadedFile,
        ToolbarIcon::Loop => GuiMessage::ToggleLoopPlayback,
        ToolbarIcon::Random => GuiMessage::PlayRandomSampleRange,
        ToolbarIcon::Play => GuiMessage::PlaySelectedSample,
        ToolbarIcon::Stop => GuiMessage::StopPlayback,
    }
}

static FOCUS_LOADED_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="3" y="3" width="2" height="2"/>
  <rect x="6" y="3.25" width="7" height="1.5"/>
  <rect x="3" y="7" width="2" height="2"/>
  <rect x="6" y="7.25" width="7" height="1.5"/>
  <rect x="3" y="11" width="2" height="2"/>
  <rect x="6" y="11.25" width="7" height="1.5"/>
</svg>"#,
);

static LOOP_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M4 3h5.4V1.5L14 5l-4.6 3.5V7H4.2C3 7 2 8 2 9.2V10H.5v-.8C.5 5.8 2 3 4 3z"/>
  <path d="M12 13H6.6v1.5L2 11l4.6-3.5V9H12c1.2 0 2-1 2-2.2V6h1.5v.8C15.5 10.2 14 13 12 13z"/>
</svg>"#,
);

static RANDOM_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path d="M2 4h2.1c1.8 0 2.9.8 4.1 2.5l.8 1.1c.8 1.1 1.4 1.4 2.6 1.4H12V7l3 3-3 3v-2h-.4c-1.9 0-3.1-.7-4.2-2.4l-.8-1.1C5.8 6.3 5.2 6 4.1 6H2z"/>
  <path d="M11.6 4H12V2l3 3-3 3V6h-.4c-1.2 0-1.8.3-2.6 1.4l-.2.3-.9-1.4.5-.7C8.5 4.7 9.7 4 11.6 4z"/>
  <path d="M2 10h2.1c1.1 0 1.7-.3 2.5-1.5l.9 1.4c-1 1.4-2 2.1-3.4 2.1H2z"/>
</svg>"#,
);

static PLAY_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <polygon points="4,3 13,8 4,13"/>
</svg>"#,
);

static STOP_ICON: ui::SvgIconTintCache = ui::SvgIconTintCache::new(
    r#"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <rect x="4" y="4" width="8" height="8"/>
</svg>"#,
);
