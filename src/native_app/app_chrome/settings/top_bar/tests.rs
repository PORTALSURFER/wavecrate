use super::projection::{
    AUDIO_ENGINE_TOOLTIP, GENERAL_SETTINGS_TOOLTIP, HELP_TOOLTIPS_ACTIVE_TOOLTIP,
    RELEASE_UPDATE_TOOLTIP, TopControlBarProjection, VOLUME_SLIDER_TOOLTIP,
};
use super::{GENERAL_SETTINGS_BUTTON_ID, SETTINGS_GEAR_ICON_SVG, settings_gear_icon};
use crate::native_app::test_support::state::{AppSettingsTab, NativeAppStateFixture};
use radiant::{
    prelude::{Point, Rect, Vector2},
    runtime::PaintPrimitive,
};

#[test]
fn top_control_bar_projection_keeps_product_copy_and_volume() {
    let mut state = NativeAppStateFixture::default().build();
    state.audio.volume = 0.42;

    let projection = TopControlBarProjection::from_app_state(&state);

    assert_eq!(projection.volume.value, 0.42);
    assert_eq!(projection.volume.tooltip, VOLUME_SLIDER_TOOLTIP);
    assert_eq!(
        projection.settings_controls.audio_engine.tooltip,
        AUDIO_ENGINE_TOOLTIP
    );
    assert_eq!(
        projection.settings_controls.general_settings.tooltip,
        GENERAL_SETTINGS_TOOLTIP
    );
    assert_eq!(
        projection.settings_controls.help_tooltips.active_tooltip,
        HELP_TOOLTIPS_ACTIVE_TOOLTIP
    );
    assert_eq!(
        projection.settings_controls.release_update.tooltip,
        RELEASE_UPDATE_TOOLTIP
    );
    assert!(!projection.settings_controls.release_update.visible);
}

#[test]
fn top_control_bar_projection_marks_open_audio_settings_tab_active() {
    let mut state = NativeAppStateFixture::default().build();
    state.ui.settings.ui.audio_settings_open = true;
    state.ui.settings.ui.app_settings_tab = AppSettingsTab::AudioEngine;

    let projection = TopControlBarProjection::from_app_state(&state);

    assert!(projection.settings_controls.audio_engine.active);
    assert!(!projection.settings_controls.general_settings.active);
}

#[test]
fn top_control_bar_projection_marks_open_general_settings_tab_active() {
    let mut state = NativeAppStateFixture::default().build();
    state.ui.settings.ui.audio_settings_open = true;
    state.ui.settings.ui.app_settings_tab = AppSettingsTab::General;

    let projection = TopControlBarProjection::from_app_state(&state);

    assert!(!projection.settings_controls.audio_engine.active);
    assert!(projection.settings_controls.general_settings.active);
}

#[test]
fn top_control_bar_projection_keeps_closed_settings_controls_inactive() {
    let mut state = NativeAppStateFixture::default().build();
    state.ui.settings.ui.audio_settings_open = false;
    state.ui.settings.ui.app_settings_tab = AppSettingsTab::AudioEngine;

    let projection = TopControlBarProjection::from_app_state(&state);

    assert!(!projection.settings_controls.audio_engine.active);
    assert!(!projection.settings_controls.general_settings.active);
}

#[test]
fn top_control_bar_projection_carries_help_tooltip_mode() {
    let mut state = NativeAppStateFixture::default().build();
    state.ui.chrome.help_tooltips_enabled = true;

    let projection = TopControlBarProjection::from_app_state(&state);

    assert!(projection.help_tooltips_enabled);
    assert!(projection.settings_controls.help_tooltips_enabled);
    assert!(projection.settings_controls.help_tooltips.active);
}

#[test]
fn top_control_bar_projection_lights_release_update_indicator_when_available() {
    let mut state = NativeAppStateFixture::default().build();
    state
        .ui
        .release_update
        .finish(Ok(Some(wavecrate::updater::PublicReleaseInfo {
            build_id: String::from("wavecrate-nightly-b999-test"),
            build_number: 999,
            version: String::from("nightly"),
            released_at: String::from("2026-06-25T20:13:25.000Z"),
            download_page_url: String::from("https://portalsurfer.org/wavecrate/"),
        })));

    let projection = TopControlBarProjection::from_app_state(&state);

    assert!(projection.settings_controls.release_update.visible);
    assert!(projection.settings_controls.release_update.active);
}

#[test]
fn settings_gear_icon_uses_cog_silhouette_with_center_hole() {
    assert!(SETTINGS_GEAR_ICON_SVG.contains(r#"fill-rule="evenodd""#));
    assert!(SETTINGS_GEAR_ICON_SVG.contains("M8 5.6a2.4 2.4"));
    assert!(!SETTINGS_GEAR_ICON_SVG.contains("<circle"));
    assert!(!SETTINGS_GEAR_ICON_SVG.contains("stroke-width"));

    let icon = settings_gear_icon(false);
    let mut primitives = Vec::<PaintPrimitive>::new();
    icon.append_paint(
        &mut primitives,
        GENERAL_SETTINGS_BUTTON_ID,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 16.0)),
    );

    assert_eq!(
        primitives
            .iter()
            .filter_map(PaintPrimitive::svg)
            .filter(|svg| svg.widget_id == GENERAL_SETTINGS_BUTTON_ID)
            .count(),
        1
    );
}
