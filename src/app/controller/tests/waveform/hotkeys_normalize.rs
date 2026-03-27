use super::super::super::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
};
use super::super::common::max_sample_amplitude;
use crate::app::controller::AppController;
use crate::app::controller::ui::hotkeys;
use crate::app::state::{DestructiveSelectionEdit, FocusContext};
use crate::selection::SelectionRange;
use std::path::Path;
use std::time::{Duration, Instant};

fn pump_background_jobs_until(
    controller: &mut AppController,
    mut predicate: impl FnMut(&mut AppController) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        controller.poll_background_jobs();
        if predicate(controller) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for background job condition");
}

#[test]
fn t_hotkey_prompts_trim_selection_in_waveform_focus() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "trim_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "trim_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "trim-selection")
        .unwrap();
    controller.handle_hotkey(action, FocusContext::Waveform);

    assert!(controller.ui.waveform.pending_destructive.is_some());
    assert_eq!(
        controller
            .ui
            .waveform
            .pending_destructive
            .as_ref()
            .unwrap()
            .edit,
        DestructiveSelectionEdit::TrimSelection
    );
}

#[test]
fn slash_hotkeys_prompt_fade_selection_in_waveform_focus() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "fade_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "fade_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let backslash = hotkeys::iter_actions()
        .find(|action| action.id == "fade-selection-left-to-right")
        .unwrap();
    controller.handle_hotkey(backslash, FocusContext::Waveform);
    assert_eq!(
        controller
            .ui
            .waveform
            .pending_destructive
            .as_ref()
            .unwrap()
            .edit,
        DestructiveSelectionEdit::FadeLeftToRight
    );

    controller.ui.waveform.pending_destructive = None;

    let slash = hotkeys::iter_actions()
        .find(|action| action.id == "fade-selection-right-to-left")
        .unwrap();
    controller.handle_hotkey(slash, FocusContext::Waveform);
    assert_eq!(
        controller
            .ui
            .waveform
            .pending_destructive
            .as_ref()
            .unwrap()
            .edit,
        DestructiveSelectionEdit::FadeRightToLeft
    );
}

#[test]
fn enter_hotkey_commits_edit_fades_without_exporting() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "apply_edit_fades_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "apply_edit_fades_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );
    let edit_selection = SelectionRange::new(0.25, 0.75).with_fade_out(0.5, 0.0);
    controller.set_edit_selection_range(edit_selection);

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "commit-waveform-edit-fades")
        .unwrap();
    let export_before = controller.ui.waveform.selection_export_flash_nonce;
    let apply_before = controller.ui.waveform.edit_selection_apply_flash_nonce;

    controller.handle_hotkey(action, FocusContext::Waveform);

    assert_eq!(controller.ui.waveform.selection_export_flash_nonce, export_before);
    assert_eq!(
        controller.ui.waveform.edit_selection_apply_flash_nonce,
        apply_before + 1
    );
    let updated = controller
        .ui
        .waveform
        .edit_selection
        .expect("edit selection after apply");
    assert!(!updated.has_edit_effects());
}

#[test]
fn m_hotkey_prompts_mute_selection_in_waveform_focus() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "mute_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "mute_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "mute-selection")
        .unwrap();
    controller.handle_hotkey(action, FocusContext::Waveform);

    assert_eq!(
        controller
            .ui
            .waveform
            .pending_destructive
            .as_ref()
            .unwrap()
            .edit,
        DestructiveSelectionEdit::MuteSelection
    );
}

#[test]
fn n_hotkey_prompts_normalize_selection_when_selection_present() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "normalize_select_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "normalize_select_hotkey.wav",
        &[0.0, 0.2, -0.6, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "normalize-waveform")
        .unwrap();
    controller.handle_hotkey(action, FocusContext::Waveform);

    assert_eq!(
        controller
            .ui
            .waveform
            .pending_destructive
            .as_ref()
            .unwrap()
            .edit,
        DestructiveSelectionEdit::NormalizeSelection
    );
}

#[test]
fn n_hotkey_normalizes_whole_loaded_sample_when_no_selection() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "normalize_full_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "normalize_full_hotkey.wav",
        &[0.1, -0.5, 0.25],
        SelectionRange::new(0.0, 0.5),
    );
    controller.ui.waveform.selection = None;

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "normalize-waveform")
        .unwrap();
    controller.handle_hotkey(action, FocusContext::Waveform);

    assert!(controller.ui.waveform.pending_destructive.is_none());
    let peak = max_sample_amplitude(&wav_path);
    assert!((peak - 1.0).abs() < 1e-4, "peak={peak}");
}

#[test]
fn c_hotkey_prompts_crop_selection_in_waveform_focus() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "crop_hotkey.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    load_waveform_selection(
        &mut controller,
        &source,
        "crop_hotkey.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "crop-selection")
        .unwrap();
    controller.handle_hotkey(action, FocusContext::Waveform);

    assert_eq!(
        controller
            .ui
            .waveform
            .pending_destructive
            .as_ref()
            .unwrap()
            .edit,
        DestructiveSelectionEdit::CropSelection
    );
}

#[test]
fn shift_c_hotkey_crops_selection_to_new_sample() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "original.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = load_waveform_selection(
        &mut controller,
        &source,
        "original.wav",
        &[0.0, 0.1, 0.2, 0.3],
        SelectionRange::new(0.25, 0.75),
    );

    let action = hotkeys::iter_actions()
        .find(|action| action.id == "crop-selection-new-sample")
        .unwrap();
    controller.handle_hotkey(action, FocusContext::Waveform);

    let cropped_path = source.root.join("original_crop001.wav");
    pump_background_jobs_until(&mut controller, |controller| {
        cropped_path.is_file()
            && controller
                .sample_view
                .wav
                .loaded_audio
                .as_ref()
                .is_some_and(|audio| audio.relative_path == Path::new("original_crop001.wav"))
    });
    assert!(cropped_path.is_file());
    let original_samples: Vec<f32> = hound::WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(original_samples, vec![0.0, 0.1, 0.2, 0.3]);

    let cropped_samples: Vec<f32> = hound::WavReader::open(&cropped_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(cropped_samples, vec![0.1, 0.2]);
}
