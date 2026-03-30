use super::*;
use crate::app::controller::test_support::{dummy_controller, write_test_wav};
use crate::app_core::actions::NativeUiAction;
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn setup_native_looping_controller(selection: SelectionRange) -> Option<AppController> {
    let mut player = crate::audio::AudioPlayer::playing_for_tests()?;
    let dir = tempdir().ok()?;
    let wav_path = dir.path().join("native_loop_selection.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);
    let bytes: std::sync::Arc<[u8]> = std::fs::read(&wav_path).ok()?.into();
    let duration = 30.0;
    player.set_audio(bytes.clone(), duration);

    let (mut controller, source) = dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("native_loop_selection.wav"),
        bytes,
        duration_seconds: duration,
        sample_rate: 8,
    });
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));
    controller.ui.waveform.loop_enabled = true;
    let _ = controller.play_audio(true, None);
    controller.is_playing().then_some(controller)
}

#[test]
fn native_waveform_selection_update_retargets_loop_playback_after_cycle() {
    let Some(mut controller) = setup_native_looping_controller(SelectionRange::new(0.1, 0.4))
    else {
        return;
    };
    controller.ui.waveform.playhead.position = 0.3;

    controller.apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
        start_micros: 200_000,
        end_micros: 600_000,
        snap_override: false,
        preserve_view_edge: false,
    });

    let pending = controller
        .audio
        .pending_loop_retarget
        .as_mut()
        .expect("loop retarget scheduled");
    assert!((pending.start_override - 0.2).abs() < 1e-6);
    pending.deadline = Instant::now() - Duration::from_millis(1);

    controller.tick_playhead();

    assert!((controller.ui.waveform.playhead.position - 0.2).abs() < 1e-6);
    assert!(controller.audio.pending_loop_retarget.is_none());
}

#[test]
fn play_from_start_auditions_focused_review_slice_without_selection() {
    let Some(mut controller) = setup_native_looping_controller(SelectionRange::new(0.1, 0.4))
    else {
        return;
    };
    controller.selection_state.range.set_range(None);
    controller.apply_selection(None);
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.05, 0.1),
        SelectionRange::new(0.42, 0.48),
    ];
    controller.start_slice_review();
    controller.move_slice_review_focus(1);

    controller.apply_native_ui_action(NativeUiAction::PlayFromStart);

    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!((controller.ui.waveform.playhead.position - 0.42).abs() < 1e-6);
    assert_eq!(controller.ui.waveform.playhead.active_span_end, Some(0.48));
}

#[test]
fn fine_slide_native_action_bypasses_slice_review_navigation() {
    let (mut controller, _source) = dummy_controller();
    controller.set_loaded_audio_duration_for_tests(1.0);
    controller.sample_view.waveform.decoded =
        Some(std::sync::Arc::new(crate::waveform::DecodedWaveform {
            cache_token: 1,
            samples: std::sync::Arc::from(vec![0.0; 8]),
            analysis_samples: std::sync::Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 48_000,
            channels: 1,
        }));
    controller.set_selection_range(SelectionRange::new(0.25, 0.5));
    controller.ui.waveform.slices =
        vec![SelectionRange::new(0.1, 0.2), SelectionRange::new(0.3, 0.4)];
    controller.start_slice_review();

    controller.apply_native_ui_action(NativeUiAction::SlideWaveformSelection {
        delta: 1,
        fine: true,
    });

    let moved = controller
        .ui
        .waveform
        .selection
        .expect("selection should remain active");
    assert!((moved.start() - 0.375).abs() < 1.0e-6);
    assert!((moved.end() - 0.625).abs() < 1.0e-6);
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(0));

    controller.apply_native_ui_action(NativeUiAction::MoveWaveformSliceFocus { delta: 1 });
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(1));
}
