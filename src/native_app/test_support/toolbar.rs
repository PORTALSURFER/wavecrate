use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::toolbar as chrome_toolbar;
use crate::native_app::app_chrome::view_models::toolbar::MainToolbarViewModel;
use crate::native_app::ui::ids as widget_ids;
use radiant::prelude as ui;

pub(in crate::native_app) use chrome_toolbar::{
    ToolbarControlProjection, ToolbarIcon, ToolbarProjection,
};

pub(in crate::native_app) const TOOLBAR_FOCUS_LOADED_ID: u64 =
    chrome_toolbar::TOOLBAR_FOCUS_LOADED_ID;
pub(in crate::native_app) const TOOLBAR_APPLY_EDIT_MARK_EDITS_ID: u64 =
    chrome_toolbar::TOOLBAR_APPLY_EDIT_MARK_EDITS_ID;
pub(in crate::native_app) const TOOLBAR_RANDOM_ID: u64 = chrome_toolbar::TOOLBAR_RANDOM_ID;
pub(in crate::native_app) const TOOLBAR_METRONOME_ID: u64 = widget_ids::TOOLBAR_METRONOME_ID;
pub(in crate::native_app) const TOOLBAR_SIMILAR_SECTIONS_ID: u64 =
    chrome_toolbar::TOOLBAR_SIMILAR_SECTIONS_ID;
pub(in crate::native_app) const TOOLBAR_STOP_ID: u64 = chrome_toolbar::TOOLBAR_STOP_ID;
pub(in crate::native_app) const TOOLBAR_ZERO_CROSSING_SNAP_ID: u64 =
    chrome_toolbar::TOOLBAR_ZERO_CROSSING_SNAP_ID;
pub(in crate::native_app) const TOOLBAR_BPM_SNAP_ID: u64 = widget_ids::TOOLBAR_BPM_SNAP_ID;
pub(in crate::native_app) const TOOLBAR_BEAT_GUIDE_COUNT_ID: u64 =
    widget_ids::TOOLBAR_BEAT_GUIDE_COUNT_ID;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MainToolbarProjection {
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
        similar_sections_available: model.similar_sections_available,
        similar_sections_enabled: model.similar_sections_enabled,
        sticky_random_sample_range_playback: model.sticky_random_sample_range_playback,
        loop_playback: model.loop_playback,
        playing: model.playing,
        zero_crossing_snap_enabled: model.zero_crossing_snap_enabled,
        bpm_snap_enabled: model.bpm_snap_enabled,
        beat_guides_enabled: model.beat_guides_enabled,
        metronome_enabled: model.metronome_enabled,
        beat_guide_count: model.beat_guide_count,
        pending_edit_mark_edits: model.pending_edit_mark_edits,
    }
}

pub(in crate::native_app) fn toolbar_icon_button(
    id: u64,
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::View<GuiMessage> {
    chrome_toolbar::toolbar_icon_button(id, icon, enabled, active)
}

pub(in crate::native_app) fn toolbar_icon_color(enabled: bool, active: bool) -> ui::Rgba8 {
    chrome_toolbar::toolbar_icon_color(enabled, active)
}

pub(in crate::native_app) fn toolbar_icon_glyph(
    icon: ToolbarIcon,
    enabled: bool,
    active: bool,
) -> ui::SvgIcon {
    chrome_toolbar::toolbar_icon_glyph(icon, enabled, active)
}
