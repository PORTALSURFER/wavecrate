use super::*;
use crate::app::controller::jobs::{FileOpResult, UndoFileJob};
use crate::app::controller::test_support;
use crate::app::controller::undo::{UndoOutcome, UndoStack};
use crate::app::controller::undo_jobs::run_undo_file_job;
use crate::sample_sources::Rating;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};

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
fn request_loop_crossfade_prompt_rejects_non_wav_targets_with_explicit_message() {
    let (mut controller, source) =
        test_support::prepare_with_source_and_wav_entries(vec![test_support::sample_entry(
            "clip.flac",
            Rating::NEUTRAL,
        )]);
    std::fs::write(source.root.join("clip.flac"), b"not-a-wav").expect("write flac fixture");

    let err = controller
        .request_loop_crossfade_prompt_for_browser_row(0)
        .expect_err("non-wav loop crossfade should fail");

    assert_eq!(
        err,
        "Seamless loop crossfade only supports WAV files; .flac is not supported"
    );
    assert!(controller.ui.loop_crossfade_prompt.is_none());
}

#[test]
fn apply_loop_crossfade_prompt_creates_suffixed_copy_preserves_tag_and_selects_result() {
    let (mut controller, source, _absolute_path) =
        prepare_loop_crossfade_controller("clip.wav", Rating::KEEP_1);
    let write_fence = Arc::new(crate::app::controller::jobs::SourceRemapWriteFence::default());
    let snapshot_path = source.root.join("loop-crossfade-remap.staged");
    let source_db = SourceDatabase::open_for_source_write(&source.root).expect("source db");
    let fence = source_db
        .snapshot_to_path_with_write_fence(&snapshot_path)
        .expect("snapshot fence");
    assert!(write_fence.install(fence));
    controller.runtime.source_lane.pending_remap =
        Some(crate::app::controller::state::runtime::PendingSourceRemap {
            request_id: 73,
            source: source.clone(),
            new_root: tempfile::tempdir().expect("remap target").keep(),
            queued_at: std::time::Instant::now(),
            canceled: false,
            write_fence: Arc::clone(&write_fence),
        });
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

    assert!(write_fence.is_canceled());
    assert!(
        controller
            .runtime
            .source_lane
            .pending_remap
            .as_ref()
            .is_some_and(|pending| pending.canceled)
    );
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
    assert_eq!(
        controller
            .database_for(&source)
            .expect("source db")
            .looped_for_path(&expected_relative)
            .expect("copied looped metadata"),
        Some(true)
    );
    let copied_index = controller
        .wav_index_for_path(&expected_relative)
        .expect("copied wav index");
    assert!(
        controller
            .wav_entry(copied_index)
            .expect("copied wav entry")
            .looped
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

#[test]
fn loop_crossfade_undo_removes_looped_metadata_for_generated_copy() {
    let (mut controller, source, _absolute_path) =
        prepare_loop_crossfade_controller("clip.wav", Rating::KEEP_1);
    controller.ui.loop_crossfade_prompt = Some(LoopCrossfadePrompt {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("clip.wav"),
        settings: LoopCrossfadeSettings::default(),
    });

    controller
        .apply_loop_crossfade_prompt()
        .expect("loop crossfade should apply");

    let expected_relative = PathBuf::from("clip_fade5ms.wav");
    controller.undo();
    let undo_job = match controller
        .history
        .pending_undo
        .as_ref()
        .map(|pending| &pending.job)
    {
        Some(job) => job.clone(),
        None => panic!("expected deferred undo entry"),
    };
    let undo_result = run_undo_file_job(undo_job, Arc::new(AtomicBool::new(false)), None);
    controller.apply_file_op_result(FileOpResult::UndoFile(undo_result));
    assert_eq!(
        controller
            .database_for(&source)
            .expect("source db")
            .looped_for_path(&expected_relative)
            .expect("looped lookup after undo"),
        None
    );
    assert!(controller.wav_index_for_path(&expected_relative).is_none());
}

#[test]
fn loop_crossfade_redo_restores_cached_looped_and_playback_metadata() {
    let (mut controller, source, _absolute_path) =
        prepare_loop_crossfade_controller("clip.wav", Rating::KEEP_1);
    controller.ui.loop_crossfade_prompt = Some(LoopCrossfadePrompt {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("clip.wav"),
        settings: LoopCrossfadeSettings::default(),
    });

    controller
        .apply_loop_crossfade_prompt()
        .expect("loop crossfade should apply");

    let expected_relative = PathBuf::from("clip_fade5ms.wav");
    controller
        .database_for(&source)
        .expect("source db")
        .set_last_played_at(&expected_relative, 42)
        .expect("set restored playback age");
    let entry_index = controller
        .wav_index_for_path(&expected_relative)
        .expect("generated wav index");
    controller
        .wav_entries
        .entry_mut(entry_index)
        .expect("generated wav entry")
        .last_played_at = Some(42);

    let mut stack = mem::replace(&mut controller.history.undo_stack, UndoStack::new(32));
    let undo_pending = match stack.undo(&mut controller).expect("undo should queue") {
        UndoOutcome::Deferred(pending) => *pending,
        UndoOutcome::Applied(label) => panic!("expected deferred undo, got applied {label}"),
        UndoOutcome::Empty => panic!("expected deferred undo entry"),
    };
    controller.history.undo_stack = stack;
    let undo_job = undo_pending.job.clone();
    controller.history.pending_undo = Some(undo_pending);
    let undo_result = run_undo_file_job(undo_job, Arc::new(AtomicBool::new(false)), None);
    controller.apply_file_op_result(FileOpResult::UndoFile(undo_result));
    assert!(controller.wav_index_for_path(&expected_relative).is_none());

    let mut stack = mem::replace(&mut controller.history.undo_stack, UndoStack::new(32));
    let redo_pending = match stack.redo(&mut controller).expect("redo should queue") {
        UndoOutcome::Deferred(pending) => *pending,
        UndoOutcome::Applied(label) => panic!("expected deferred redo, got applied {label}"),
        UndoOutcome::Empty => panic!("expected deferred redo entry"),
    };
    controller.history.undo_stack = stack;
    let redo_job = redo_pending.job.clone();
    controller.history.pending_undo = Some(redo_pending);
    let redo_result = run_undo_file_job(redo_job, Arc::new(AtomicBool::new(false)), None);
    controller.apply_file_op_result(FileOpResult::UndoFile(redo_result));

    let db = controller.database_for(&source).expect("source db");
    assert_eq!(
        db.looped_for_path(&expected_relative)
            .expect("looped lookup after redo"),
        Some(true)
    );
    assert_eq!(
        db.last_played_at_for_path(&expected_relative)
            .expect("playback lookup after redo"),
        Some(42)
    );
    let restored_index = controller
        .wav_index_for_path(&expected_relative)
        .expect("restored wav index");
    let restored_entry = controller
        .wav_entry(restored_index)
        .expect("restored entry");
    assert!(restored_entry.looped);
    assert_eq!(restored_entry.last_played_at, Some(42));
}
