use super::*;

#[test]
fn browser_focus_hotkey_uses_explicit_scope() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.loaded_wav = Some("two.wav".into());
    controller.ui.focus.set_context(FocusContext::Waveform);

    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Shell(
                    crate::app_core::actions::NativeShellAction::FocusLoadedSampleInBrowser
                )
            )
        }),
        FocusContext::SampleBrowser,
    );

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn compare_anchor_hotkey_sets_focused_sample_anchor() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(1);

    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample)
            )
        }),
        FocusContext::SampleBrowser,
    );

    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("two")
    );
    assert_eq!(
        controller
            .sample_view
            .wav
            .compare_anchor
            .as_ref()
            .map(|anchor| anchor.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn browser_scoped_hotkey_is_ignored_when_no_section_is_focused() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row(0);
    let before = controller.ui.browser.selection.selected_paths.clone();

    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Browser(
                    crate::app_core::actions::NativeBrowserAction::ToggleFocusedBrowserRowSelection
                )
            )
        }),
        FocusContext::None,
    );

    assert_eq!(controller.ui.browser.selection.selected_paths, before);
}

#[test]
fn browser_focus_move_hotkey_moves_the_selected_row() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);

    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Browser(
                    crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { delta: 1 }
                )
            )
        }),
        FocusContext::SampleBrowser,
    );

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
}
