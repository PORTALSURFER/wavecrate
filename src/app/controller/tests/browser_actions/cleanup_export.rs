use super::super::super::test_support::{
    dummy_controller, load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry,
    write_test_wav,
};
use crate::app::controller::library::selection_export::SelectionClipExportRequest;
use crate::app_core::controller::AppController;
use crate::sample_sources::SampleSource;
use crate::sample_sources::SourceDatabase;
use crate::selection::SelectionRange;
use crate::waveform::WaveformRenderer;
use hound::WavReader;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tempfile::tempdir;

#[test]
fn exporting_selection_updates_entries_and_db() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());

    let orig = root.join("orig.wav");
    write_test_wav(&orig, &[0.0, 0.25, 0.5, 0.75]);

    controller
        .load_waveform_for_selection(&source, Path::new("orig.wav"))
        .unwrap();

    let entry = controller
        .export_selection_clip(SelectionClipExportRequest {
            source_id: &source.id,
            relative_path: Path::new("orig.wav"),
            bounds: SelectionRange::new(0.0, 0.5),
            target_tag: Some(crate::sample_sources::Rating::KEEP_1),
            add_to_browser: true,
            register_in_source: true,
        })
        .unwrap();

    assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_1);
    assert_eq!(entry.relative_path, PathBuf::from("orig_selection_001.wav"));
    assert_eq!(controller.wav_entries_len(), 1);
    assert_eq!(controller.ui.browser.viewport.visible.len(), 1);
    let exported_path = root.join(&entry.relative_path);
    assert!(exported_path.exists());
    let exported: Vec<f32> = WavReader::open(&exported_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(exported, vec![0.0, 0.25]);

    let db = controller.database_for(&source).unwrap();
    let rows = db.list_files().unwrap();
    let saved = rows
        .iter()
        .find(|row| row.relative_path == entry.relative_path)
        .unwrap();
    assert_eq!(saved.tag, crate::sample_sources::Rating::KEEP_1);
}

#[test]
fn browser_normalize_resumes_playback_when_playing() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "normalize_resume_browser.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    load_waveform_selection(
        &mut controller,
        &source,
        "normalize_resume_browser.wav",
        &[0.0, 0.2, -0.6, 0.3],
        SelectionRange::new(0.0, 1.0),
    );
    if controller.play_audio(false, None).is_err() || !controller.is_playing() {
        return;
    }
    controller.ui.waveform.playhead.position = 0.5;

    assert!(controller.normalize_browser_sample(0).is_ok());

    assert!(controller.is_playing());
    assert!((controller.ui.waveform.playhead.position - 0.5).abs() < 1e-6);
}

#[test]
fn pruning_missing_browser_sample_keeps_remaining_rows_visible() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    write_test_wav(&source.root.join("alive.wav"), &[0.0, 0.1, -0.1]);
    let db = SourceDatabase::open(&source.root).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(
            Path::new("alive.wav"),
            1,
            1,
            "hash-alive",
            crate::sample_sources::Rating::NEUTRAL,
            false,
        )
        .unwrap();
    batch
        .upsert_file_with_hash_and_tag(
            Path::new("gone.wav"),
            1,
            1,
            "hash-gone",
            crate::sample_sources::Rating::NEUTRAL,
            false,
        )
        .unwrap();
    batch.commit().unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("alive.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("gone.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.prune_missing_sample(&source, Path::new("gone.wav"))?;

    assert_eq!(controller.wav_entries_len(), 1);
    assert!(
        controller
            .wav_entries
            .lookup
            .contains_key(Path::new("alive.wav"))
    );
    assert!(
        !controller
            .wav_entries
            .lookup
            .contains_key(Path::new("gone.wav"))
    );
    assert!(db.entry_for_path(Path::new("gone.wav")).unwrap().is_none());
    Ok(())
}

#[test]
fn deleting_browser_sample_moves_focus_forward() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    for name in ["a.wav", "b.wav", "c.wav"] {
        write_test_wav(&source.root.join(name), &[0.1, -0.1]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("c.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(1);

    controller.delete_browser_sample(1)?;

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("c.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));

    controller.delete_browser_sample(1)?;

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    Ok(())
}
