use super::*;

#[test]
fn play_hotkeys_route_start_and_playhead_positions() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
    load_waveform_selection(
        &mut controller,
        &source,
        "one.wav",
        &[0.1, -0.2, 0.3, -0.4],
        SelectionRange::new(0.0, 0.5),
    );
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.37;
    controller.ui.waveform.cursor = Some(0.22);

    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Transport(
                    crate::app_core::actions::NativeTransportAction::PlayFromStart
                )
            )
        }),
        FocusContext::SampleBrowser,
    );
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.0));

    controller.ui.waveform.playhead.position = 0.37;
    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Transport(
                    crate::app_core::actions::NativeTransportAction::PlayFromCurrentPlayhead
                )
            )
        }),
        FocusContext::SampleBrowser,
    );
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.37));
}

#[test]
fn compare_anchor_play_hotkey_routes_global_compare_replay() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("current.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("anchor.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("current.wav"), &[0.0, -0.1]);
    controller.focus_browser_row_only(0);
    controller.set_compare_anchor_from_focused_browser_sample();
    controller.focus_browser_row_only(1);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.handle_hotkey(
        action_for(|action| {
            matches!(
                action,
                crate::app_core::actions::NativeUiAction::Transport(
                    crate::app_core::actions::NativeTransportAction::PlayCompareAnchor
                )
            )
        }),
        FocusContext::Waveform,
    );

    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("compare replay should queue");
    assert_eq!(pending.relative_path, PathBuf::from("anchor.wav"));
    assert!(pending.force_loaded_audio);
}
