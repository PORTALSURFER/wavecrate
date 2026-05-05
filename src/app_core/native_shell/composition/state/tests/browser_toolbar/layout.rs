use super::*;

#[test]
fn browser_action_buttons_stay_inside_toolbar() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_edit_pills = true;
    model.browser_actions.can_delete = true;
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let toolbar = browser_toolbar_layout(&layout, &style, &model);
        let buttons = browser_action_buttons(&layout, &style, &model, &toolbar);
        assert_eq!(buttons.len(), 3);
        assert_eq!(buttons[0].label, "Random");
        assert_eq!(buttons[0].icon, Some(WaveformToolbarIcon::Dice));
        assert!(buttons[0].enabled);
        assert!(!buttons[0].active);
        assert_eq!(buttons[1].label, "Cleanup");
        assert_eq!(buttons[1].icon, Some(WaveformToolbarIcon::Filter));
        assert!(buttons[1].enabled);
        assert!(!buttons[1].active);
        assert_eq!(buttons[2].label, "Tags");
        assert_eq!(buttons[2].icon, None);
        assert!(buttons[2].enabled);
        assert!(!buttons[2].active);
        assert_rect_inside(layout.browser_toolbar, buttons[0].rect);
        assert_rect_inside(layout.browser_toolbar, buttons[1].rect);
        assert_rect_inside(layout.browser_toolbar, buttons[2].rect);
    }
}

#[test]
fn browser_toolbar_controls_do_not_overlap_action_cluster() {
    let mut model = AppModel::default();
    model.browser_actions.can_rename = true;
    model.browser_actions.can_edit_pills = true;
    model.browser_actions.can_delete = true;
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style, &model);
    let buttons = browser_action_buttons(&layout, &style, &model, &controls);
    assert_eq!(buttons.len(), 3);
    assert!(
        controls
            .rating_filter_chips
            .iter()
            .all(|chip| chip.width() > 1.0)
    );
    assert_rect_inside(layout.browser_toolbar, controls.search_field);
    assert!(
        controls.search_field.max.x <= layout.browser_toolbar.max.x - style.sizing.text_inset_x
    );
    assert_eq!(buttons[0].rect, controls.action_slots[0]);
    assert_eq!(buttons[1].rect, controls.action_slots[1]);
    assert_eq!(buttons[2].rect, controls.action_slots[2]);
    assert!(controls.rating_filter_chips[7].max.x <= buttons[0].rect.min.x);
    assert!(buttons[0].rect.max.x <= buttons[1].rect.min.x);
    assert!(buttons[1].rect.max.x <= controls.search_field.min.x);
    assert!(controls.search_field.max.x <= buttons[2].rect.min.x);
    assert!(controls.search_field.width() < layout.browser_toolbar.width());
    assert!(controls.activity_chip.width() <= 0.0);
    assert!(controls.sort_chip.width() <= 0.0);
    assert!(
        controls
            .triage_chips
            .into_iter()
            .all(|chip| chip.width() <= 0.0)
    );
}

#[test]
fn browser_toolbar_places_playback_age_chips_between_rating_and_mark_controls() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style, &AppModel::default());
    let rating_gap = controls.rating_filter_chips[1].min.x - controls.rating_filter_chips[0].max.x;
    let age_gap =
        controls.playback_age_filter_chips[1].min.x - controls.playback_age_filter_chips[0].max.x;
    let rating_to_age_gap =
        controls.playback_age_filter_chips[0].min.x - controls.rating_filter_chips[7].max.x;
    let age_to_mark_gap =
        controls.marked_filter_chip.min.x - controls.playback_age_filter_chips[2].max.x;

    assert!(
        controls
            .playback_age_filter_chips
            .iter()
            .all(|chip| chip.width() > 1.0)
    );
    assert!(rating_to_age_gap > rating_gap);
    assert!(age_to_mark_gap > age_gap);
    assert!(controls.marked_filter_chip.max.x <= controls.action_slots[0].min.x);
}

#[test]
fn browser_toolbar_right_side_does_not_hit_search_field() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let model = AppModel::default();
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style, &model);
    let point = Point::new(
        (controls.search_field.max.x + layout.browser_toolbar.max.x) * 0.5,
        (layout.browser_toolbar.min.y + layout.browser_toolbar.max.y) * 0.5,
    );
    assert!(point.x > controls.search_field.max.x);
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        None
    );
}

#[test]
fn browser_toolbar_tags_button_sits_right_of_search_field() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let controls = browser_toolbar_layout(&layout, &style, &AppModel::default());
    assert!(controls.action_slots[2].width() > 1.0);
    assert!(controls.search_field.width() > 1.0);
    assert!(controls.search_field.max.x <= controls.action_slots[2].min.x);
}

#[test]
fn open_pill_editor_shrinks_browser_row_hit_area_and_focuses_sidebar_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.browser.pill_editor.open = true;
    model
        .browser
        .rows
        .push(BrowserRowModel::new(0, "Kick 01", 1, false, true));
    model.browser.visible_count = model.browser.rows.len();
    let mut state = NativeShellState::new();

    let input_rect = state
        .browser_pill_editor_input_rect(&layout, &model)
        .expect("sidebar input should render");
    let point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );

    assert_eq!(state.browser_row_at_point(&layout, &model, point), None);
    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::FocusBrowserPillEditorInput)
    );

    let rows = state.cached_browser_rows(&layout, &style, &model);
    assert!(rows.iter().all(|row| row.rect.max.x <= input_rect.min.x));
}

#[test]
fn pill_editor_auto_rename_button_maps_to_toggle_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.browser.pill_editor.open = true;
    let mut state = NativeShellState::new();
    let input_rect = state
        .browser_pill_editor_input_rect(&layout, &model)
        .expect("sidebar input should render");
    let field_height = style.sizing.browser_row_height.max(22.0);
    let point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        input_rect.min.y - 4.0 - field_height * 0.5,
    );

    assert_eq!(
        state.browser_action_at_point(&layout, &model, point, false),
        Some(UiAction::ToggleBrowserPillEditorPrimaryAction)
    );
}

#[test]
fn top_bar_controls_fit_inside_control_row() {
    for viewport in [
        Vector2::new(820.0, 520.0),
        Vector2::new(1280.0, 720.0),
        Vector2::new(2300.0, 1080.0),
    ] {
        let layout = ShellLayout::build(viewport);
        let style = style_for_layout(&layout);
        let controls = resolve_top_bar_surface_layout(
            layout.top_bar,
            style.sizing,
            &top_bar_surface_content(&AppModel::default()),
        );
        assert_rect_inside(controls.title_cluster, controls.volume_meter_rect);
        assert_rect_inside(controls.title_cluster, controls.volume_value_rect);
        assert_rect_inside(controls.title_cluster, controls.volume_label_rect);
        assert!(controls.volume_meter_rect.max.x <= controls.volume_value_rect.min.x);
        assert!(controls.volume_value_rect.max.x <= controls.volume_label_rect.min.x);
    }
}
