use super::*;
#[test]
fn waveform_toolbar_relative_grid_button_uses_highlight_when_enabled() {
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
    let relative_off = buttons_off
        .iter()
        .find(|button| button.label == "Rel Grid")
        .expect("relative bpm grid toolbar button should be present");
    assert_eq!(relative_off.text_color, style.text_muted);

    model.waveform_chrome.relative_bpm_grid_enabled = true;
    let buttons_on = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let relative_on = buttons_on
        .iter()
        .find(|button| button.label == "Rel Grid")
        .expect("relative bpm grid toolbar button should be present");
    assert_eq!(relative_on.text_color, style.accent_warning);
    assert_eq!(
        relative_on.action,
        Some(UiAction::SetRelativeBpmGridEnabled { enabled: false })
    );
}

#[test]
fn waveform_toolbar_normalized_audition_button_uses_highlight_when_enabled() {
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
    let normalized_off = buttons_off
        .iter()
        .find(|button| button.label == "Norm")
        .expect("normalized audition toolbar button should be present");
    assert_eq!(normalized_off.text_color, style.text_muted);

    model.waveform_chrome.normalized_audition_enabled = true;
    let buttons_on = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let normalized_on = buttons_on
        .iter()
        .find(|button| button.label == "Norm")
        .expect("normalized audition toolbar button should be present");
    assert_eq!(normalized_on.text_color, style.accent_warning);
}

#[test]
fn waveform_toolbar_toggle_buttons_share_warning_accent_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform_chrome.transient_snap_enabled = true;
    model.waveform_chrome.transient_markers_enabled = true;
    model.waveform_chrome.slice_mode_enabled = true;
    model.waveform.loop_enabled = true;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );

    for label in ["Tr Snap", "Show Tr", "Slice", "Loop"] {
        let button = buttons
            .iter()
            .find(|button| button.label == label)
            .unwrap_or_else(|| panic!("{label} toolbar button should be present"));
        assert_eq!(button.text_color, style.accent_warning);
    }
}

#[test]
fn waveform_toolbar_locked_loop_on_uses_warning_accent_with_lock_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform.loop_enabled = true;
    model.waveform_chrome.loop_lock_enabled = true;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button = buttons
        .iter()
        .find(|button| button.label == "Loop")
        .expect("loop toolbar button should be present");

    assert_eq!(button.overlay_icon, Some(WaveformToolbarIcon::Lock));
    assert_eq!(button.text_color, style.accent_warning);
}

#[test]
fn waveform_toolbar_locked_loop_off_stays_muted_with_lock_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform.loop_enabled = false;
    model.waveform_chrome.loop_lock_enabled = true;

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button = buttons
        .iter()
        .find(|button| button.label == "Loop")
        .expect("loop toolbar button should be present");

    assert_eq!(button.overlay_icon, Some(WaveformToolbarIcon::Lock));
    assert_eq!(button.text_color, style.text_muted);
}

#[test]
fn waveform_toolbar_silence_split_button_uses_blue_accent() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&AppModel::default()),
        false,
        None,
    );
    let button = buttons
        .iter()
        .find(|button| button.label == "Silence Split")
        .expect("silence split toolbar button should be present");

    assert_eq!(button.text_color, style.highlight_blue_soft);
}

#[test]
fn waveform_toolbar_silence_split_button_emits_detect_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button = buttons
        .iter()
        .find(|button| button.label == "Silence Split")
        .expect("silence split toolbar button should be present");

    assert_eq!(button.action, Some(UiAction::DetectWaveformSilenceSlices));
    assert!(button.enabled);
}

#[test]
fn waveform_toolbar_exact_dedupe_button_uses_blue_accent_and_detect_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button = buttons
        .iter()
        .find(|button| button.label == "Exact Dedupe")
        .expect("exact dedupe toolbar button should be present");

    assert_eq!(button.text_color, style.highlight_blue_soft);
    assert_eq!(
        button.action,
        Some(UiAction::DetectWaveformExactDuplicateSlices)
    );
    assert!(button.enabled);
}

#[test]
fn waveform_toolbar_clean_dups_button_requires_duplicate_cleanup_batch() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = AppModel::default();
    model.waveform.loaded_label = Some(String::from("kick.wav"));

    let buttons = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button = buttons
        .iter()
        .find(|button| button.label == "Clean Dups")
        .expect("clean dups toolbar button should be present");

    assert_eq!(
        button.action,
        Some(UiAction::CleanWaveformExactDuplicateSlices)
    );
    assert!(!button.enabled);

    model.waveform_chrome.slice_mode_enabled = true;
    model
        .waveform
        .slices
        .push(crate::compat_app_contract::WaveformSlicePreviewModel {
            range: crate::compat_app_contract::NormalizedRangeModel::new(180, 420),
            selected: false,
            focused: false,
            marked_for_export: false,
            duplicate_cleanup_candidate: false,
            duplicate_cleanup_exempted: false,
        });
    let buttons_enabled = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button_enabled = buttons_enabled
        .iter()
        .find(|button| button.label == "Clean Dups")
        .expect("clean dups toolbar button should be present");

    assert!(!button_enabled.enabled);

    model.waveform_chrome.exact_duplicate_cleanup_available = true;
    let buttons_cleanup = waveform_toolbar_buttons(
        &layout,
        &style,
        &NativeMotionModel::from_app_model(&model),
        false,
        None,
    );
    let button_cleanup = buttons_cleanup
        .iter()
        .find(|button| button.label == "Clean Dups")
        .expect("clean dups toolbar button should be present");

    assert!(button_cleanup.enabled);
}
