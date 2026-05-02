use super::*;

#[test]
fn toolbar_hit_test_focuses_browser_search() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let search_field = state
        .browser_search_field_rect(&layout, &model)
        .expect("browser search field should be present");
    let point = Point::new(
        (search_field.min.x + search_field.max.x) * 0.5,
        (search_field.min.y + search_field.max.y) * 0.5,
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(crate::compat_app_contract::UiAction::FocusBrowserSearch)
    );
}

#[test]
fn toolbar_hit_test_toggles_browser_rating_filter_chip() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 rating filter chip should be present");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(
            crate::compat_app_contract::UiAction::ToggleBrowserRatingFilter {
                level: 3,
                invert: false,
            }
        )
    );
}

#[test]
fn toolbar_hit_test_alt_click_inverts_browser_rating_filter_chip() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 4)
        .expect("locked keep rating filter chip should be present");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, true),
        Some(
            crate::compat_app_contract::UiAction::ToggleBrowserRatingFilter {
                level: 4,
                invert: true,
            }
        )
    );
}

#[test]
fn toolbar_hit_test_toggles_browser_playback_age_filter_chip() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let chip = state
        .browser_playback_age_filter_chip_rect(
            &layout,
            &model,
            crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanMonth,
        )
        .expect("month playback-age filter chip should be present");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(
            crate::compat_app_contract::UiAction::ToggleBrowserPlaybackAgeFilter {
                bucket: crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanMonth,
                invert: false,
            }
        )
    );
}

#[test]
fn toolbar_hit_test_alt_click_inverts_browser_playback_age_filter_chip() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let chip = state
        .browser_playback_age_filter_chip_rect(
            &layout,
            &model,
            crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanWeek,
        )
        .expect("week playback-age filter chip should be present");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, true),
        Some(
            crate::compat_app_contract::UiAction::ToggleBrowserPlaybackAgeFilter {
                bucket: crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanWeek,
                invert: true,
            }
        )
    );
}

#[test]
fn toolbar_hit_test_ignores_empty_right_host_area() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let search_field = state
        .browser_search_field_rect(&layout, &model)
        .expect("browser search field should be present");
    let point = Point::new(
        (search_field.max.x + layout.browser_toolbar.max.x) * 0.5,
        (layout.browser_toolbar.min.y + layout.browser_toolbar.max.y) * 0.5,
    );
    assert!(point.x > search_field.max.x);
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        None
    );
}

#[test]
fn browser_toolbar_exposes_no_column_chip_hit_targets() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.columns[2].item_count = 42;
    assert!(state.browser_column_chip_rect(&layout, &model, 2).is_none());
}

#[test]
fn waveform_toolbar_hit_test_emits_transport_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.transport_running = false;
    let play = state
        .waveform_toolbar_button_rect(&layout, &model, "Play")
        .expect("play waveform toolbar button should be present");
    let point = Point::new(
        (play.min.x + play.max.x) * 0.5,
        (play.min.y + play.max.y) * 0.5,
    );
    assert_eq!(
        state.waveform_toolbar_action_at_point(&layout, &model, point),
        Some(crate::compat_app_contract::UiAction::ToggleTransport)
    );
}

#[test]
fn waveform_toolbar_hit_test_emits_stop_action_when_transport_running() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.transport_running = true;
    let stop = state
        .waveform_toolbar_button_rect(&layout, &model, "Stop")
        .expect("stop waveform toolbar button should be present");
    let point = Point::new(
        (stop.min.x + stop.max.x) * 0.5,
        (stop.min.y + stop.max.y) * 0.5,
    );
    assert_eq!(
        state.waveform_toolbar_action_at_point(&layout, &model, point),
        Some(crate::compat_app_contract::UiAction::HandleEscape)
    );
}

#[test]
fn waveform_toolbar_hit_test_emits_loop_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.waveform.loop_enabled = true;
    let loop_button = state
        .waveform_toolbar_button_rect(&layout, &model, "Loop")
        .expect("loop waveform toolbar button should be present");
    let point = Point::new(
        (loop_button.min.x + loop_button.max.x) * 0.5,
        (loop_button.min.y + loop_button.max.y) * 0.5,
    );
    assert_eq!(
        state.waveform_toolbar_action_at_point(&layout, &model, point),
        Some(crate::compat_app_contract::UiAction::ToggleLoopPlayback)
    );
}

#[test]
fn waveform_toolbar_shift_click_emits_loop_lock_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let loop_button = state
        .waveform_toolbar_button_rect(&layout, &model, "Loop")
        .expect("loop waveform toolbar button should be present");
    let point = Point::new(
        (loop_button.min.x + loop_button.max.x) * 0.5,
        (loop_button.min.y + loop_button.max.y) * 0.5,
    );
    assert_eq!(
        state.waveform_toolbar_action_at_point_with_modifiers(&layout, &model, point, true),
        Some(crate::compat_app_contract::UiAction::ToggleLoopLock)
    );
}

#[test]
fn waveform_toolbar_hit_test_emits_relative_grid_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = crate::compat_app_contract::AppModel::default();
    let relative_grid = state
        .waveform_toolbar_button_rect(&layout, &model, "Rel Grid")
        .expect("relative grid waveform toolbar button should be present");
    let point = Point::new(
        (relative_grid.min.x + relative_grid.max.x) * 0.5,
        (relative_grid.min.y + relative_grid.max.y) * 0.5,
    );
    assert_eq!(
        state.waveform_toolbar_action_at_point(&layout, &model, point),
        Some(crate::compat_app_contract::UiAction::SetRelativeBpmGridEnabled { enabled: true })
    );
}

#[test]
fn waveform_toolbar_bpm_value_widget_exposes_input_hit_target() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = crate::compat_app_contract::AppModel::default();
    model.waveform.tempo_label = Some(String::from("128.0 BPM"));
    let bpm_value = state
        .waveform_toolbar_button_rect(&layout, &model, "BPM Value")
        .expect("bpm value waveform toolbar widget should be present");
    let point = Point::new(
        (bpm_value.min.x + bpm_value.max.x) * 0.5,
        (bpm_value.min.y + bpm_value.max.y) * 0.5,
    );
    assert!(state.waveform_bpm_input_at_point(&layout, &model, point));
}
