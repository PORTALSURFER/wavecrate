use super::super::test_support::{prepare_with_source_and_wav_entries, sample_entry, write_test_wav};
use crate::app::controller::state::audio::LoadedAudio;
use crate::app::controller::state::selection::CompareAnchorSample;
use crate::app::state::{CompareAnchorState, FocusContext, StatusTone};
use crate::sample_sources::{Rating, SampleSource};
use std::path::{Path, PathBuf};

fn second_source(controller: &mut crate::app::controller::AppController) -> SampleSource {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("source_two");
    std::mem::forget(temp);
    std::fs::create_dir_all(&root).expect("source_two root");
    let source = SampleSource::new(root);
    controller.library.sources.push(source.clone());
    source
}

#[test]
fn setting_compare_anchor_stores_focused_sample_identity() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", Rating::NEUTRAL),
        sample_entry("current.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(1);
    controller.set_compare_anchor_from_focused_browser_sample();

    assert_eq!(
        controller.sample_view.wav.compare_anchor,
        Some(CompareAnchorSample {
            source_id: controller
                .selection_state
                .ctx
                .selected_source
                .clone()
                .expect("selected source"),
            relative_path: PathBuf::from("current.wav"),
        })
    );
    assert_eq!(
        controller.ui.compare_anchor,
        Some(CompareAnchorState {
            source_id: controller
                .selection_state
                .ctx
                .selected_source
                .clone()
                .expect("selected source"),
            relative_path: PathBuf::from("current.wav"),
            label: String::from("current"),
        })
    );
    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("current")
    );
}

#[test]
fn compare_anchor_replay_queues_anchor_without_stealing_focus() {
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

    controller.play_compare_anchor();

    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending compare playback");
    assert_eq!(pending.source_id, source.id);
    assert_eq!(pending.relative_path, PathBuf::from("anchor.wav"));
    assert!(pending.force_loaded_audio);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("current.wav"))
    );
}

#[test]
fn compare_anchor_replay_supports_cross_source_reference() {
    let (mut controller, source_one) =
        prepare_with_source_and_wav_entries(vec![sample_entry("current.wav", Rating::NEUTRAL)]);
    let source_two = second_source(&mut controller);
    write_test_wav(&source_one.root.join("current.wav"), &[0.0, 0.2]);
    write_test_wav(&source_two.root.join("anchor.wav"), &[0.0, -0.2]);

    controller.focus_browser_row_only(0);
    controller.sample_view.wav.compare_anchor = Some(CompareAnchorSample {
        source_id: source_two.id.clone(),
        relative_path: PathBuf::from("anchor.wav"),
    });
    controller.ui.compare_anchor = Some(CompareAnchorState {
        source_id: source_two.id.clone(),
        relative_path: PathBuf::from("anchor.wav"),
        label: String::from("anchor.wav"),
    });
    controller.ui.waveform.compare_anchor_label = Some(String::from("anchor.wav"));

    controller.play_compare_anchor();

    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending compare playback");
    assert_eq!(pending.source_id, source_two.id);
    assert_eq!(pending.relative_path, PathBuf::from("anchor.wav"));
    assert!(pending.force_loaded_audio);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("current.wav"))
    );
}

#[test]
fn compare_anchor_replay_requires_anchor() {
    let (mut controller, _source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("current.wav", Rating::NEUTRAL)]);

    controller.play_compare_anchor();

    assert_eq!(controller.ui.status.text, "Set a compare anchor first");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn compare_anchor_replay_clears_missing_anchor() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("current.wav", Rating::NEUTRAL)]);
    controller.sample_view.wav.compare_anchor = Some(CompareAnchorSample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("missing.wav"),
    });
    controller.ui.compare_anchor = Some(CompareAnchorState {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("missing.wav"),
        label: String::from("missing.wav"),
    });
    controller.ui.waveform.compare_anchor_label = Some(String::from("missing.wav"));

    controller.play_compare_anchor();

    assert!(controller.sample_view.wav.compare_anchor.is_none());
    assert!(controller.ui.compare_anchor.is_none());
    assert!(controller.ui.waveform.compare_anchor_label.is_none());
    assert_eq!(
        controller.ui.status.text,
        "Compare anchor file is no longer available"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn browser_playback_returns_to_focused_sample_after_compare_replay() {
    let (mut controller, source_one) =
        prepare_with_source_and_wav_entries(vec![sample_entry("current.wav", Rating::NEUTRAL)]);
    let source_two = second_source(&mut controller);
    controller.ui.focus.set_context(FocusContext::SampleBrowser);
    controller.focus_browser_row_only(0);
    controller.selection_state.ctx.selected_source = Some(source_one.id.clone());
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("current.wav"));
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source_two.id.clone(),
        root: source_two.root.clone(),
        relative_path: PathBuf::from("anchor.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 1.0,
        sample_rate: 48_000,
    });
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("anchor.wav"));
    controller.ui.loaded_wav = Some(PathBuf::from("anchor.wav"));
    controller.sample_view.wav.compare_anchor = Some(CompareAnchorSample {
        source_id: source_two.id.clone(),
        relative_path: PathBuf::from("anchor.wav"),
    });
    controller.ui.compare_anchor = Some(CompareAnchorState {
        source_id: source_two.id.clone(),
        relative_path: PathBuf::from("anchor.wav"),
        label: String::from("anchor.wav"),
    });
    controller.ui.waveform.compare_anchor_label = Some(String::from("anchor.wav"));
    write_test_wav(&source_one.root.join("current.wav"), &[0.0, 0.3]);

    controller.play_compare_anchor();
    controller.runtime.jobs.pending_playback = None;

    assert!(controller.play_audio(false, None).is_ok());

    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("focused-sample playback should queue");
    assert_eq!(pending.source_id, source_one.id);
    assert_eq!(pending.relative_path, PathBuf::from("current.wav"));
    assert!(!pending.force_loaded_audio);
}
