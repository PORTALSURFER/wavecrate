use super::*;
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use crate::app_core::actions::NativeUiAction;

#[test]
fn replay_from_last_start_requeues_pending_playback() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marker.wav");
    controller.select_wav_by_path(Path::new("marker.wav"));
    controller.record_play_start(0.42);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.1;

    let handled = controller.replay_from_last_start();
    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(
        pending.start_override,
        controller.ui.waveform.last_start_marker.map(f64::from)
    );
}

#[test]
fn play_from_start_requeues_zero_position_without_selection() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "start.wav");
    controller.select_wav_by_path(Path::new("start.wav"));
    controller.record_play_start(0.42);
    controller.ui.waveform.cursor = Some(0.25);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.6;

    let handled = controller.play_from_start();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(0.0));
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.0));
}

#[test]
fn play_from_start_prefers_active_play_selection_start() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marked.wav");
    controller.select_wav_by_path(Path::new("marked.wav"));
    let selection = crate::selection::SelectionRange::new(0.25, 0.6);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);
    controller.ui.waveform.cursor = Some(0.1);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.8;

    let handled = controller.play_from_start();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(pending.start_override, Some(f64::from(selection.start())));
    assert_eq!(
        controller.ui.waveform.last_start_marker,
        Some(selection.start())
    );
}

#[test]
fn play_from_start_preserves_zoomed_view_inside_active_selection() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "zoomed-marked.wav");
    controller.select_wav_by_path(Path::new("zoomed-marked.wav"));
    install_decoded_waveform(&mut controller);
    let selection = crate::selection::SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.55,
        end: 0.65,
    };

    let handled = controller.play_from_start();

    assert!(handled);
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.start_override),
        Some(Some(f64::from(selection.start())))
    );
    assert_eq!(
        controller.ui.waveform.last_start_marker,
        Some(selection.start())
    );
    assert_eq!(controller.ui.waveform.cursor, Some(selection.start()));
    assert!(
        (controller.ui.waveform.view.start - 0.55).abs() < 1.0e-9,
        "playback start should preserve the current zoomed view start"
    );
    assert!(
        (controller.ui.waveform.view.end - 0.65).abs() < 1.0e-9,
        "playback start should preserve the current zoomed view end"
    );
}

#[test]
fn replay_from_last_start_falls_back_to_cursor() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "marker.wav");
    controller.select_wav_by_path(Path::new("marker.wav"));
    controller.ui.waveform.cursor = Some(0.25);
    controller.ui.waveform.last_start_marker = None;

    let handled = controller.replay_from_last_start();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(
        pending.start_override,
        controller.ui.waveform.cursor.map(f64::from)
    );
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.25));
}

#[test]
fn play_from_current_playhead_prefers_visible_playhead_position() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "playhead.wav");
    controller.select_wav_by_path(Path::new("playhead.wav"));
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.58;
    controller.ui.waveform.cursor = Some(0.33);
    controller.ui.waveform.last_start_marker = Some(0.1);

    let handled = controller.play_from_current_playhead();

    assert!(handled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert_eq!(
        pending.start_override,
        Some(f64::from(controller.ui.waveform.playhead.position))
    );
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.58));
}

#[test]
fn play_from_waveform_cursor_prefers_cursor_position_over_visible_playhead() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "click-play.wav");
    controller.select_wav_by_path(Path::new("click-play.wav"));
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.84;

    controller.apply_native_ui_action(NativeUiAction::SetWaveformCursorPrecise {
        position_nanos: 330_000_000,
    });
    controller.apply_native_ui_action(NativeUiAction::PlayFromWaveformCursor);

    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback request");
    assert!(
        (pending.start_override.expect("playback start override") - 0.33).abs() < 1.0e-6
    );
    assert!(
        (controller
            .ui
            .waveform
            .last_start_marker
            .expect("last start marker") as f64
            - 0.33)
            .abs()
            < 1.0e-6
    );
    assert_eq!(controller.ui.waveform.playhead.position, 0.84);
}
