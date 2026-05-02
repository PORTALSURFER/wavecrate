use super::*;
use crate::compat_app_contract::AutomationNodeSnapshot;

fn child<'a>(parent: &'a AutomationNodeSnapshot, id: &str) -> &'a AutomationNodeSnapshot {
    parent
        .children
        .iter()
        .find(|node| node.id.0 == id)
        .unwrap_or_else(|| panic!("missing automation child {id}"))
}

#[test]
fn browser_random_action_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button = state
        .browser_action_button_rect(&layout, &model, "Random")
        .expect("random button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleRandomNavigationMode)
    );
}

#[test]
fn browser_cleanup_action_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button = state
        .browser_action_button_rect(&layout, &model, "Cleanup")
        .expect("cleanup button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleBrowserDuplicateCleanupMode)
    );
}

#[test]
fn browser_tags_action_button_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button = state
        .browser_action_button_rect(&layout, &model, "Tags")
        .expect("tags button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleBrowserPillEditor)
    );
}

#[test]
fn browser_action_helpers_share_retained_interaction_geometry() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();

    assert!(state.browser_action_hit_test_cache_key.is_none());
    let button = state
        .browser_action_button_rect(&layout, &model, "Random")
        .expect("random button should render");
    let retained_key = state.browser_action_hit_test_cache_key;
    assert!(retained_key.is_some());
    assert!(state.browser_toolbar_layout.is_some());
    assert!(!state.browser_action_buttons.is_empty());

    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleRandomNavigationMode)
    );
    assert_eq!(state.browser_action_hit_test_cache_key, retained_key);
}

#[test]
fn browser_action_cache_invalidates_when_toolbar_model_state_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    let mut state = NativeShellState::new();

    let _ = state
        .browser_action_button_rect(&layout, &model, "Tags")
        .expect("tags button should render");
    let closed_key = state
        .browser_action_hit_test_cache_key
        .expect("closed toolbar state should populate action cache");

    model.browser_actions.pill_editor_open = true;
    let _ = state
        .browser_action_button_rect(&layout, &model, "Tags")
        .expect("tags button should still render");

    assert_ne!(
        state.browser_action_hit_test_cache_key,
        Some(closed_key),
        "toolbar state changes must refresh the retained action geometry snapshot"
    );
}

#[test]
fn browser_marked_filter_chip_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let chip = state
        .browser_marked_filter_chip_rect(&layout, &model)
        .expect("marked filter chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleBrowserMarkedFilter)
    );
}

#[test]
fn browser_derived_label_filter_chip_click_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let chip = state
        .browser_derived_label_filter_chip_rect(&layout, &model)
        .expect("derived-label filter chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleBrowserDerivedLabelFilter { invert: false })
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, true),
        Some(UiAction::ToggleBrowserDerivedLabelFilter { invert: true })
    );
}

#[test]
fn browser_automation_exposes_marked_filter_and_marked_row_metadata() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.browser.marked_filter_active = true;
    model.browser.rows.push(
        BrowserRowModel::new(0, "Marked row", 1, false, true)
            .with_marked(true)
            .with_bucket_label("165 BPM"),
    );
    model.browser.visible_count = model.browser.rows.len();
    let mut state = NativeShellState::new();

    let snapshot = state.automation_snapshot(&layout, &model);
    let browser = child(&snapshot.root, "browser.panel");
    let marked_filter = child(browser, "browser.marked_filter");
    let derived_label_filter = child(browser, "browser.derived_label_filter");
    let table = child(browser, "browser.table");
    let row = child(table, "browser.row.0");

    assert_eq!(
        marked_filter.role,
        crate::compat_app_contract::AutomationRole::Button
    );
    assert!(marked_filter.selected);
    assert_eq!(
        marked_filter.available_actions,
        vec![String::from("toggle_browser_marked_filter")]
    );
    assert_eq!(
        derived_label_filter.role,
        crate::compat_app_contract::AutomationRole::Button
    );
    assert_eq!(
        derived_label_filter.available_actions,
        vec![String::from("toggle_browser_derived_label_filter")]
    );
    assert_eq!(row.metadata.get("marked").map(String::as_str), Some("true"));
}

#[test]
fn browser_automation_exposes_playback_age_filters_and_row_bucket_metadata() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.browser.active_playback_age_filters = [true, false, true];
    model.browser.rows.push(
        BrowserRowModel::new(0, "Never played row", 1, false, true)
            .with_playback_age_bucket(crate::compat_app_contract::PlaybackAgeBucket::NeverPlayed),
    );
    model.browser.visible_count = model.browser.rows.len();
    let mut state = NativeShellState::new();

    let snapshot = state.automation_snapshot(&layout, &model);
    let browser = child(&snapshot.root, "browser.panel");
    let never_filter = child(browser, "browser.playback_age_filter.never");
    let week_filter = child(browser, "browser.playback_age_filter.week");
    let table = child(browser, "browser.table");
    let row = child(table, "browser.row.0");

    assert!(never_filter.selected);
    assert!(week_filter.selected);
    assert_eq!(
        never_filter.available_actions,
        vec![String::from("toggle_browser_playback_age_filter")]
    );
    assert_eq!(
        row.metadata.get("playback_age_bucket").map(String::as_str),
        Some("never")
    );
}
