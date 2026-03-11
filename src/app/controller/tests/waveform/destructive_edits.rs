use super::super::super::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::library::selection_edits::SelectionEditRequest;
use crate::app::state::{DestructiveSelectionEdit, WaveformView};
use crate::app_core::state::StatusTone;
use crate::selection::SelectionRange;
use hound::WavReader;
use std::path::Path;

#[test]
fn align_waveform_start_uses_hover_cursor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "align.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("align.wav");
    write_test_wav(&wav_path, &[1.0, 2.0, 3.0, 4.0]);
    controller
        .load_waveform_for_selection(&source, Path::new("align.wav"))
        .unwrap();
    controller.set_waveform_cursor_from_hover(0.5);
    controller.ui.waveform.last_start_marker = None;

    controller.align_waveform_start_to_last_marker().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![3.0, 4.0, 1.0, 2.0]);
}

#[test]
fn click_removal_interpolates_selected_span() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "click.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let selection = SelectionRange::new(0.4, 0.6);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "click.wav",
        &[0.0, 1.0, 9.0, -1.0, 0.0],
        selection,
    );
    let preserved_view = WaveformView {
        start: 0.2,
        end: 0.4,
    };
    controller.ui.waveform.view = preserved_view;

    controller.repair_clicks_selection().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert!(samples[2].abs() < 1e-6);
    assert_eq!(controller.ui.waveform.selection, Some(selection));
    assert!((controller.ui.waveform.view.start - preserved_view.start).abs() < 1e-6);
    assert!((controller.ui.waveform.view.end - preserved_view.end).abs() < 1e-6);
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn cropping_selection_overwrites_file() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "edit.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "edit.wav",
        &[0.1, 0.2, 0.3, 0.4],
        SelectionRange::new(0.25, 0.75),
    );

    controller.crop_waveform_selection().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.2, 0.3]);
    assert!(controller.ui.waveform.selection.is_none());
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
}

#[test]
fn trimming_selection_removes_span() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "trim.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "trim.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    controller.trim_waveform_selection().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.0, 0.3]);
    assert!(controller.ui.waveform.selection.is_none());
    let entry = controller.wav_entry(0).unwrap();
    assert!(entry.file_size > 0);
}

#[test]
fn destructive_edit_request_prompts_without_yolo_mode() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "warn.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "warn.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let outcome = controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection)
        .unwrap();

    assert!(matches!(outcome, SelectionEditRequest::Prompted));
    assert!(controller.ui.waveform.pending_destructive.is_some());
    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples.len(), 4);
}

#[test]
fn yolo_mode_applies_destructive_edit_immediately() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "yolo.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "yolo.wav",
        &[0.1, 0.2, 0.3, 0.4],
        SelectionRange::new(0.25, 0.75),
    );
    controller.set_destructive_yolo_mode(true);

    let outcome = controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::CropSelection)
        .unwrap();

    assert!(matches!(outcome, SelectionEditRequest::Applied));
    assert!(controller.ui.waveform.pending_destructive.is_none());
    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.2, 0.3]);
}

#[test]
fn confirming_pending_destructive_edit_clears_prompt() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "confirm.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "confirm.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );
    controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::TrimSelection)
        .unwrap();
    let prompt = controller.ui.waveform.pending_destructive.clone().unwrap();

    controller.apply_confirmed_destructive_edit(prompt.edit);

    assert!(controller.ui.waveform.pending_destructive.is_none());
    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.0, 0.3]);
}
