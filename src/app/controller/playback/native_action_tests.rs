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
