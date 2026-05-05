use super::*;

#[test]
fn waveform_toolbar_icon_buttons_use_uniform_hit_cell_widths() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = false;
    let labels = [
        "Channel",
        "Norm",
        "BPM Snap",
        "Rel Grid",
        "Tr Snap",
        "Show Tr",
        "Slice",
        "Silence Split",
        "Exact Dedupe",
        "Clean Dups",
        "Loop",
        "Compare",
        "Play",
        "Rec",
    ];
    let widths: Vec<u32> = labels
        .iter()
        .map(|label| {
            let rect = state
                .waveform_toolbar_button_rect(&layout, &model, label)
                .unwrap_or_else(|| panic!("missing waveform toolbar button rect for {label}"));
            (rect.width() * 100.0).round() as u32
        })
        .collect();
    let min_width = widths.iter().copied().min().unwrap_or(0);
    let max_width = widths.iter().copied().max().unwrap_or(0);
    assert!(
        max_width.saturating_sub(min_width) <= 100,
        "toolbar widths diverged too far: {widths:?}"
    );
}

#[test]
fn waveform_toolbar_renders_without_per_button_rect_chrome() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.transport_running = false;
    let mut state = NativeShellState::new();
    let button_rects = ["Channel", "Compare", "Play"]
        .into_iter()
        .map(|label| {
            state
                .waveform_toolbar_button_rect(&layout, &model, label)
                .unwrap_or_else(|| panic!("missing waveform toolbar button rect for {label}"))
        })
        .collect::<Vec<_>>();
    let frame = state.build_frame(&layout, &model);
    for button_rect in button_rects {
        assert!(!frame.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Rect(FillRect { rect, .. }) if *rect == button_rect)
        }));
    }
}

#[test]
fn waveform_toolbar_click_sets_flash_in_chrome_motion_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
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
    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert_eq!(
        fingerprint.flashed_waveform_toolbar_hint,
        Some(WaveformToolbarHoverHint::Play)
    );
    assert!(fingerprint.waveform_toolbar_flash_ticks > 0);
}

#[test]
fn waveform_toolbar_compare_button_requires_anchor() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let buttons_disabled = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&AppModel::default()),
        false,
        None,
    );
    let disabled = buttons_disabled
        .iter()
        .find(|button| button.label == "Compare")
        .expect("compare toolbar button should be present");
    assert!(!disabled.enabled);
    assert_eq!(disabled.action, Some(UiAction::PlayCompareAnchor));
    assert_eq!(disabled.text_color, style.text_muted);

    let mut model = AppModel::default();
    model.waveform_chrome.compare_anchor_available = true;
    model.waveform_chrome.compare_anchor_label = Some(String::from("anchor.wav"));
    let buttons_enabled = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let enabled = buttons_enabled
        .iter()
        .find(|button| button.label == "Compare")
        .expect("compare toolbar button should be present");
    assert!(enabled.enabled);
    assert_eq!(enabled.action, Some(UiAction::PlayCompareAnchor));
    assert_eq!(enabled.text_color, style.highlight_cyan_soft);
}

#[test]
fn waveform_toolbar_compare_button_hit_testing_emits_compare_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.waveform_chrome.compare_anchor_available = true;
    model.waveform_chrome.compare_anchor_label = Some(String::from("anchor.wav"));
    let compare = state
        .waveform_toolbar_button_rect(&layout, &model, "Compare")
        .expect("compare waveform toolbar button should be present");
    let point = Point::new(
        (compare.min.x + compare.max.x) * 0.5,
        (compare.min.y + compare.max.y) * 0.5,
    );

    assert_eq!(
        state.waveform_toolbar_action_at_point(&layout, &model, point),
        Some(crate::compat_app_contract::UiAction::PlayCompareAnchor)
    );
}

#[test]
fn waveform_toolbar_hover_uses_theme_highlight_color() {
    let style = StyleTokens::for_viewport_width(1280.0);
    let expected = blend_color(
        blend_color(style.text_muted, style.bg_tertiary, 0.26),
        style.highlight_cyan,
        0.82,
    );

    assert_eq!(
        waveform_toolbar_visual_color(&style, style.highlight_cyan, true, false, true, false, 0.0,),
        expected
    );
}

#[test]
fn waveform_toolbar_play_button_uses_transport_accent_when_idle() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.transport_running = false;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let play = buttons
        .iter()
        .find(|button| button.label == "Play")
        .expect("play toolbar button should be present");

    assert_eq!(play.text_color, style.accent_warning);
}

#[test]
fn waveform_toolbar_stop_button_uses_stop_icon_and_escape_when_running() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.transport_running = true;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let stop = buttons
        .iter()
        .find(|button| button.label == "Stop")
        .expect("stop toolbar button should be present while transport runs");

    assert_eq!(stop.icon, Some(WaveformToolbarIcon::Stop));
    assert_eq!(stop.action, Some(UiAction::HandleEscape));
    assert_eq!(stop.text_color, style.highlight_orange_soft);
}

#[test]
fn waveform_toolbar_bpm_snap_button_uses_highlight_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let buttons_off = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let bpm_snap_off = buttons_off
        .iter()
        .find(|button| button.label == "BPM Snap")
        .expect("bpm snap toolbar button should be present");
    assert_eq!(bpm_snap_off.text_color, style.text_muted);

    model.waveform_chrome.bpm_snap_enabled = true;
    let buttons_on = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let bpm_snap_on = buttons_on
        .iter()
        .find(|button| button.label == "BPM Snap")
        .expect("bpm snap toolbar button should be present");
    assert_eq!(bpm_snap_on.text_color, style.accent_warning);
}
