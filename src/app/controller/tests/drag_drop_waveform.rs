use super::super::test_support::{sample_entry, write_test_wav};
use super::super::*;
use crate::app::state::TriageFlagColumn;
use crate::app::state::{DragPayload, DragSource, DragTarget};
use crate::app_core::actions::NativeUiAction;
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use crate::app_core::state::StatusTone;
use crate::selection::SelectionRange;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::tempdir;

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
fn waveform_sample_drop_copies_and_registers() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    write_test_wav(&root.join("one.wav"), &[0.1, 0.2]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.ui.drag.payload = Some(DragPayload::Sample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("one.wav"),
    });
    controller.ui.drag.origin_source = Some(DragSource::Waveform);
    controller.ui.drag.set_target(
        DragSource::Browser,
        DragTarget::BrowserTriage(TriageFlagColumn::Keep),
    );
    controller.finish_active_drag();

    assert!(root.join("one_copy001.wav").is_file());
    assert!(controller.ui.status.text.contains("Copied sample"));
}

#[test]
fn waveform_sample_drop_reports_missing_source_file() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());

    controller.ui.drag.payload = Some(DragPayload::Sample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("missing.wav"),
    });
    controller.ui.drag.origin_source = Some(DragSource::Waveform);
    controller.ui.drag.set_target(
        DragSource::Browser,
        DragTarget::BrowserTriage(TriageFlagColumn::Keep),
    );
    controller.finish_active_drag();

    assert!(controller.ui.status.text.contains("Source file missing"));
}

#[test]
/// Dropping a waveform selection onto the browser list should export a new clip.
fn waveform_selection_drop_to_browser_list_exports_selection_clip() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    write_test_wav(&root.join("loop.wav"), &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, std::path::Path::new("loop.wav"))
        .unwrap();

    controller.ui.drag.payload = Some(DragPayload::Selection {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("loop.wav"),
        bounds: SelectionRange::new(0.25, 0.75),
        keep_source_focused: true,
    });
    controller.ui.drag.origin_source = Some(DragSource::Waveform);
    controller
        .ui
        .drag
        .set_target(DragSource::Browser, DragTarget::BrowserList);
    controller.finish_active_drag();

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    pump_background_jobs_until(&mut controller, |controller| {
        root.join("loop_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });
    assert!(root.join("loop_selection_001.wav").is_file());
    assert!(controller.ui.status.text.contains("Saved clip"));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("loop.wav"))
    );
}

#[test]
/// Native E-equivalent waveform save action should export the current selection.
fn waveform_selection_native_save_exports_selection_clip() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    write_test_wav(&root.join("loop.wav"), &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, std::path::Path::new("loop.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.25, 0.75));

    controller.apply_native_ui_action(NativeUiAction::SaveWaveformSelectionToBrowser);

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    pump_background_jobs_until(&mut controller, |controller| {
        root.join("loop_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });
    assert!(root.join("loop_selection_001.wav").is_file());
    assert!(controller.ui.status.text.contains("Saved clip"));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("loop.wav"))
    );
}

#[test]
/// Native Shift+E-equivalent waveform save action should export the current selection with keep-2.
fn waveform_selection_native_save_with_keep2_exports_selection_clip() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    write_test_wav(&root.join("loop.wav"), &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, std::path::Path::new("loop.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.25, 0.75));

    controller.apply_native_ui_action(NativeUiAction::SaveWaveformSelectionToBrowserWithKeep2);

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    pump_background_jobs_until(&mut controller, |controller| {
        root.join("loop_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });
    assert!(root.join("loop_selection_001.wav").is_file());
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    let exported = rows
        .iter()
        .find(|row| row.relative_path == PathBuf::from("loop_selection_001.wav"))
        .expect("exported clip should be registered");
    assert_eq!(exported.tag, crate::sample_sources::Rating::new(2));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("loop.wav"))
    );
}
