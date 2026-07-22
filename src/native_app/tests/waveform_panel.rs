use crate::native_app::{
    app_chrome::{
        view_models::waveform_panel::WaveformPanelViewModel,
        waveform_panel::{WAVEFORM_PANEL_HEIGHT, waveform_panel},
    },
    test_support::state::NativeAppStateFixture,
    ui::ids::{
        WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID, WAVEFORM_PLAYMARK_BEAT_COUNT_ID,
        WAVEFORM_PLAYMARK_BEAT_TOGGLE_ID,
    },
};
use radiant::{
    gui::automation::AutomationRole,
    prelude::{self as ui, IntoView},
    runtime::PaintPrimitive,
};

fn loaded_sample_drag_handle_tooltip(
    state: &crate::native_app::app::NativeAppState,
) -> Option<String> {
    waveform_panel(WaveformPanelViewModel::from_app_state(state))
        .into_surface()
        .find_widget(WAVEFORM_LOADED_SAMPLE_DRAG_HANDLE_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .common()
                .tooltip
                .as_deref()
                .map(str::to_owned)
        })
}

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
fn loaded_sample_drag_handle_omits_tooltip_when_help_is_inactive() {
    let state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();

    assert_eq!(loaded_sample_drag_handle_tooltip(&state), None);
}

#[test]
fn loaded_sample_drag_handle_uses_help_tooltip_when_help_is_active() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.ui.chrome.help_tooltips_enabled = true;

    assert_eq!(
        loaded_sample_drag_handle_tooltip(&state).as_deref(),
        Some("Drag loaded sample")
    );
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

#[test]
fn playmark_beat_controls_project_shared_toolbar_state_and_semantics() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.waveform.current.set_play_selection_range(0.25, 0.75);
    state.ui.chrome.beat_guides_enabled = true;
    state.ui.chrome.beat_guide_count = 16;

    let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();
    let toggle = surface
        .find_widget(WAVEFORM_PLAYMARK_BEAT_TOGGLE_ID)
        .expect("playmark beat toggle")
        .widget_object()
        .automation_semantics();
    let count = surface
        .find_widget(WAVEFORM_PLAYMARK_BEAT_COUNT_ID)
        .expect("playmark beat count")
        .widget_object()
        .automation_semantics();

    assert_eq!(toggle.role, AutomationRole::Toggle);
    assert_eq!(toggle.label.as_deref(), Some("Playmark beat grid"));
    assert_eq!(toggle.checked, Some(true));
    assert_eq!(count.role, AutomationRole::TextInput);
    assert_eq!(count.label.as_deref(), Some("Playmark beat count"));
    assert_eq!(count.value_text.as_deref(), Some("16"));
}

#[test]
fn playmark_beat_controls_are_absent_without_a_playmark() {
    let state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();

    assert!(
        surface
            .find_widget(WAVEFORM_PLAYMARK_BEAT_TOGGLE_ID)
            .is_none()
    );
    assert!(
        surface
            .find_widget(WAVEFORM_PLAYMARK_BEAT_COUNT_ID)
            .is_none()
    );
}

#[test]
fn playmark_beat_count_is_absent_until_the_grid_is_enabled() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.waveform.current.set_play_selection_range(0.25, 0.75);

    let surface = waveform_panel(WaveformPanelViewModel::from_app_state(&state)).into_surface();
    assert!(
        surface
            .find_widget(WAVEFORM_PLAYMARK_BEAT_TOGGLE_ID)
            .is_some()
    );
    assert!(
        surface
            .find_widget(WAVEFORM_PLAYMARK_BEAT_COUNT_ID)
            .is_none()
    );
}

#[test]
fn playmark_overlay_controls_paint_in_the_waveform_viewport() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.waveform.current.set_play_selection_range(0.25, 0.75);

    let frame = waveform_panel(WaveformPanelViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(ui::Vector2::new(800.0, WAVEFORM_PANEL_HEIGHT));

    assert!(frame.paint_plan.primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.widget_id == WAVEFORM_PLAYMARK_BEAT_TOGGLE_ID && text.text.as_str() == "Grid")
    ));
}

#[test]
fn editing_playmark_label_paints_one_text_input() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.waveform.current.set_play_selection_range(0.25, 0.75);
    assert!(state.waveform.current.begin_playmark_label_edit(false, 4));

    let frame = waveform_panel(WaveformPanelViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(ui::Vector2::new(800.0, WAVEFORM_PANEL_HEIGHT));
    let inputs = frame
        .paint_plan
        .text_inputs()
        .filter(|input| input.widget_id == crate::native_app::ui::ids::WAVEFORM_PLAYMARK_LABEL_ID)
        .count();

    assert_eq!(inputs, 1);
}
