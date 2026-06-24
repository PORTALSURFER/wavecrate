use crate::native_app::{
    app_chrome::{
        view_models::waveform_panel::WaveformPanelViewModel,
        waveform_panel::{WAVEFORM_PANEL_HEIGHT, waveform_panel},
    },
    test_support::state::NativeAppStateFixture,
    ui::ids::WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID,
};
use radiant::prelude::{self as ui, IntoView};

#[test]
fn waveform_panel_omits_section_header_label() {
    let state = NativeAppStateFixture::default().build();
    let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();
    let frame = waveform_panel(WaveformPanelViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(ui::Vector2::new(800.0, WAVEFORM_PANEL_HEIGHT));

    assert!(frame.paint_plan.contains_text("No sample loaded"));
    assert!(!frame.paint_plan.contains_text("Waveform"));
    assert!(
        surface
            .find_widget(WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID)
            .is_none()
    );
    assert!(
        frame
            .paint_plan
            .stroke_polylines()
            .all(|stroke| stroke.widget_id != WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID)
    );
}

#[test]
fn loaded_waveform_title_includes_sample_drag_handle_before_name() {
    let state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();
    let frame = waveform_panel(WaveformPanelViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(ui::Vector2::new(800.0, WAVEFORM_PANEL_HEIGHT));

    assert!(
        surface
            .find_widget(WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID)
            .is_some(),
        "loaded waveform title should include interactive sample drag handle"
    );
    let handle_right_edge = frame
        .paint_plan
        .stroke_polylines()
        .filter(|stroke| stroke.widget_id == WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID)
        .flat_map(|stroke| stroke.points.iter().map(|point| point.x))
        .fold(None, |max: Option<f32>, x| {
            Some(max.map_or(x, |max| max.max(x)))
        })
        .expect("loaded waveform title should include sample drag handle");
    let title_rect = frame
        .paint_plan
        .text_runs()
        .find(|run| run.text.starts_with("synthetic-waveform |"))
        .map(|run| run.rect)
        .expect("loaded waveform title should include sample name");

    assert!(handle_right_edge < title_rect.min.x);
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
