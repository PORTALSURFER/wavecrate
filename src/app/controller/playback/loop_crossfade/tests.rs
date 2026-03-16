use super::*;
use crate::app::controller::jobs::UndoFileJob;
use crate::app::controller::test_support;
use crate::app::controller::undo::{UndoOutcome, UndoStack};
use crate::sample_sources::Rating;
use std::mem;
use std::path::{Path, PathBuf};

fn prepare_loop_crossfade_controller(
    relative_path: &str,
    tag: Rating,
) -> (AppController, SampleSource, PathBuf) {
    let (mut controller, source) =
        test_support::prepare_with_source_and_wav_entries(vec![test_support::sample_entry(
            relative_path,
            tag,
        )]);
    let absolute_path = source.root.join(relative_path);
    test_support::write_test_wav(&absolute_path, &[0.0, 0.2, 0.4, 0.1, -0.2, -0.4, -0.1, 0.3]);
    controller
        .database_for(&source)
        .expect("source db")
        .set_tag(Path::new(relative_path), tag)
        .expect("set tag");
    controller.rebuild_browser_lists();
    (controller, source, absolute_path)
}

#[test]
fn loop_crossfade_finds_low_delta_cut() {
    let samples = vec![0.0, 1.0, 2.0, 2.1, 2.2, 10.0];
    let cut = audio::find_crossfade_cut_frame(&samples, 1, 6, 2);
    assert_eq!(cut, 3);
}

#[test]
fn loop_crossfade_moves_cut_to_front() {
    let mut samples = vec![0.0, 1.0, 2.0, 2.1, 2.2, 10.0];
    audio::apply_loop_crossfade(&mut samples, 1, 6, 2).unwrap();
    let expected = [10.0, 0.0, 1.0, 2.0, 1.0, 2.2];
    for (actual, expected) in samples.iter().zip(expected.iter()) {
        assert!((actual - expected).abs() < 1.0e-6);
    }
}

#[test]
fn request_loop_crossfade_prompt_for_browser_row_sets_prompt_context() {
    let (mut controller, _source, _absolute_path) =
        prepare_loop_crossfade_controller("clip.wav", Rating::NEUTRAL);

    controller
        .request_loop_crossfade_prompt_for_browser_row(0)
        .expect("prompt should open");

    let prompt = controller
        .ui
        .loop_crossfade_prompt
        .as_ref()
        .expect("loop crossfade prompt");
    assert_eq!(prompt.relative_path, PathBuf::from("clip.wav"));
    assert_eq!(prompt.settings, LoopCrossfadeSettings::default());

    controller.clear_loop_crossfade_prompt();
    assert!(controller.ui.loop_crossfade_prompt.is_none());
}

#[test]
fn apply_loop_crossfade_prompt_creates_suffixed_copy_preserves_tag_and_selects_result() {
    let (mut controller, source, _absolute_path) =
        prepare_loop_crossfade_controller("clip.wav", Rating::KEEP_1);
    let colliding_output = source.root.join("clip_fade5ms.wav");
    test_support::write_test_wav(&colliding_output, &[0.0, 0.0, 0.0, 0.0]);
    controller.ui.loop_crossfade_prompt = Some(LoopCrossfadePrompt {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("clip.wav"),
        settings: LoopCrossfadeSettings::default(),
    });

    controller
        .apply_loop_crossfade_prompt()
        .expect("loop crossfade should apply");

    let expected_relative = PathBuf::from("clip_fade5ms_1.wav");
    let expected_absolute = source.root.join(&expected_relative);
    assert!(expected_absolute.exists(), "expected created waveform copy");
    assert!(controller.ui.loop_crossfade_prompt.is_none());
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(expected_relative.as_path())
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller
            .sample_tag_for(&source, &expected_relative)
            .expect("copied tag"),
        Rating::KEEP_1
    );
    assert!(!controller.selection_state.suppress_autoplay_once);

    let mut stack = mem::replace(&mut controller.history.undo_stack, UndoStack::new(32));
    let undo_outcome = stack.undo(&mut controller).expect("undo should queue");
    controller.history.undo_stack = stack;
    match undo_outcome {
        UndoOutcome::Deferred(deferred) => match deferred.job {
            UndoFileJob::RemoveSample {
                ref relative_path,
                ref absolute_path,
                ..
            } => {
                assert_eq!(relative_path, &expected_relative);
                assert_eq!(absolute_path, &expected_absolute);
            }
            other => panic!("unexpected undo job: {other:?}"),
        },
        UndoOutcome::Applied(label) => panic!("expected deferred undo, got applied {label}"),
        UndoOutcome::Empty => panic!("expected deferred undo entry"),
    }
}
