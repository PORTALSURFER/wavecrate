use crate::native_app::{
    app_chrome::{
        view_models::waveform_panel::WaveformPanelViewModel,
        waveform_panel::{WAVEFORM_PANEL_HEIGHT, waveform_panel},
    },
    test_support::state::NativeAppStateFixture,
};
use radiant::prelude::{self as ui, IntoView};

#[test]
fn waveform_panel_omits_section_header_label() {
    let state = NativeAppStateFixture::default().build();
    let frame = waveform_panel(WaveformPanelViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(ui::Vector2::new(800.0, WAVEFORM_PANEL_HEIGHT));

    assert!(frame.paint_plan.contains_text("No sample loaded"));
    assert!(!frame.paint_plan.contains_text("Waveform"));
}

#[test]
fn waveform_help_tooltip_attaches_to_interaction_widget() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.ui.chrome.help_tooltips_enabled = true;
    let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();
    let tooltip = surface
        .find_widget(crate::native_app::ui::ids::WAVEFORM_WIDGET_ID)
        .and_then(|widget| widget.widget_object().common().tooltip.as_deref());

    assert_eq!(
        tooltip,
        Some(
            "Waveform: click to set playback start, drag to select, Z zooms to selection, X zooms out."
        )
    );
}
