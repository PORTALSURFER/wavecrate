use super::*;

#[test]
fn waveform_toolbar_channel_button_toggles_channel_view_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    let mono_buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let mono_button = mono_buttons
        .iter()
        .find(|button| button.label == "Channel")
        .expect("channel toolbar button should be present");
    assert_eq!(
        mono_button.action,
        Some(UiAction::SetWaveformChannelView { stereo: true })
    );
    assert_eq!(mono_button.icon, Some(WaveformToolbarIcon::Mono));

    model.waveform_chrome.channel_view =
        crate::compat_app_contract::WaveformChannelViewModel::Stereo;
    let stereo_buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let stereo_button = stereo_buttons
        .iter()
        .find(|button| button.label == "Channel")
        .expect("channel toolbar button should be present");
    assert_eq!(
        stereo_button.action,
        Some(UiAction::SetWaveformChannelView { stereo: false })
    );
    assert_eq!(stereo_button.icon, Some(WaveformToolbarIcon::Stereo));
}

#[test]
fn state_overlay_renders_waveform_toolbar_hover_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let channel_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Channel")
        .expect("channel button should be present");
    let channel = Point::new(
        (channel_rect.min.x + channel_rect.max.x) * 0.5,
        (channel_rect.min.y + channel_rect.max.y) * 0.5,
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, channel),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Switch waveform view to split stereo"))
    );
}

#[test]
fn state_overlay_renders_silence_split_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));
    let mut state = NativeShellState::new();
    let button_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Silence Split")
        .expect("silence split button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Detect silence-based waveform slices"))
    );
}

#[test]
fn state_overlay_renders_exact_dedupe_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));
    let mut state = NativeShellState::new();
    let button_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Exact Dedupe")
        .expect("exact dedupe button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.text_runs.iter().any(|run| {
        run.text.contains(
            "Scan the waveform for near-duplicate hit windows using the current selection size",
        )
    }));
}

#[test]
fn state_overlay_renders_clean_dups_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));
    model.waveform_chrome.slice_mode_enabled = true;
    model.waveform_chrome.exact_duplicate_cleanup_available = true;
    model
        .waveform
        .slices
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
            selected: false,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: true,
            duplicate_cleanup_exempted: false,
        });
    let mut state = NativeShellState::new();
    let button_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Clean Dups")
        .expect("clean dups button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.text_runs.iter().any(|run| {
        run.text.contains(
            "Remove marked duplicate windows and keep the first copy plus any right-click keeps",
        )
    }));
}

#[test]
fn state_overlay_renders_relative_grid_toggle_tooltip_text() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Rel Grid")
        .expect("relative grid button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Use selection-relative BPM grid"))
    );
}

#[test]
fn state_overlay_renders_compare_tooltip_text_for_anchor_state() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.waveform_chrome.compare_anchor_available = true;
    model.waveform_chrome.compare_anchor_label = Some(String::from("anchor.wav"));
    let mut state = NativeShellState::new();
    let button_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Compare")
        .expect("compare button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Play compare anchor (anchor.wav)"))
    );
}

#[test]
fn state_overlay_renders_compare_tooltip_text_without_anchor() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let button_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Compare")
        .expect("compare button should be present");
    let point = Point::new(
        (button_rect.min.x + button_rect.max.x) * 0.5,
        (button_rect.min.y + button_rect.max.y) * 0.5,
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);

    assert!(frame.text_runs.iter().any(|run| {
        run.text
            .contains("Set a compare anchor to enable compare playback")
    }));
}
