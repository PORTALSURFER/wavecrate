use super::super::test_support::sample_entry;
use super::super::*;
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{SourceMoveResult, SourceMoveSuccess};
use crate::sample_sources::Rating;
use std::path::PathBuf;
use tempfile::tempdir;

fn cache_source_entries(
    controller: &mut AppController,
    source: &SampleSource,
    entries: Vec<WavEntry>,
) {
    let total = entries.len();
    controller
        .cache
        .wav
        .insert_page(source.id.clone(), total, total.max(1), 0, entries);
}

fn upsert_source_db_entry(controller: &mut AppController, source: &SampleSource, entry: &WavEntry) {
    let db = controller.database_for(source).unwrap();
    db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
        .unwrap();
    db.set_tag(&entry.relative_path, entry.tag).unwrap();
    db.set_looped(&entry.relative_path, entry.looped).unwrap();
    db.set_locked(&entry.relative_path, entry.locked).unwrap();
    if let Some(last_played_at) = entry.last_played_at {
        db.set_last_played_at(&entry.relative_path, last_played_at)
            .unwrap();
    }
}

#[test]
fn apply_source_move_result_invalidates_touched_sources_and_selected_target_state() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source_a");
    let target_root = temp.path().join("source_b");
    std::fs::create_dir_all(&source_root).unwrap();
    std::fs::create_dir_all(&target_root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root);
    let target = SampleSource::new(target_root);
    controller.library.sources.push(source.clone());
    controller.library.sources.push(target.clone());
    controller.selection_state.ctx.selected_source = Some(target.id.clone());
    controller.cache_db(&source).unwrap();
    controller.cache_db(&target).unwrap();
    let mut source_entry = sample_entry("one.wav", Rating::KEEP_1);
    source_entry.file_size = 7;
    source_entry.modified_ns = 11;
    source_entry.looped = true;
    source_entry.locked = true;
    source_entry.last_played_at = Some(42);
    upsert_source_db_entry(&mut controller, &source, &source_entry);

    let mut existing_target_entry = sample_entry("existing.wav", Rating::NEUTRAL);
    existing_target_entry.file_size = 5;
    existing_target_entry.modified_ns = 22;
    controller.set_wav_entries_for_tests(vec![existing_target_entry.clone()]);

    let mut moved_target_entry = sample_entry("moved.wav", Rating::KEEP_1);
    moved_target_entry.file_size = 7;
    moved_target_entry.modified_ns = 11;
    moved_target_entry.looped = true;
    moved_target_entry.locked = true;
    moved_target_entry.last_played_at = Some(42);
    upsert_source_db_entry(&mut controller, &target, &moved_target_entry);
    controller
        .database_for(&source)
        .unwrap()
        .remove_file(&source_entry.relative_path)
        .unwrap();
    cache_source_entries(&mut controller, &source, vec![source_entry]);
    cache_source_entries(&mut controller, &target, vec![existing_target_entry]);
    controller
        .ui_cache
        .browser
        .labels
        .insert(source.id.clone(), vec!["cached".into()]);
    controller
        .ui_cache
        .browser
        .labels
        .insert(target.id.clone(), vec!["cached".into()]);

    controller
        .drag_drop()
        .apply_source_move_result(SourceMoveResult {
            target_source_id: target.id.clone(),
            moved: vec![SourceMoveSuccess {
                source_id: source.id.clone(),
                relative_path: PathBuf::from("one.wav"),
                target_relative: PathBuf::from("moved.wav"),
                file_size: 1,
                modified_ns: 1,
                tag: Rating::KEEP_1,
                looped: true,
                last_played_at: Some(42),
            }],
            errors: vec!["secondary move failed".into()],
            cancelled: false,
        });

    assert!(!controller.cache.wav.entries.contains_key(&source.id));
    assert!(controller.cache.wav.entries.contains_key(&target.id));
    assert_eq!(controller.wav_entries_len(), 2);
    assert!(
        controller
            .wav_index_for_path(&PathBuf::from("existing.wav"))
            .is_some()
    );
    assert!(
        controller
            .wav_index_for_path(&PathBuf::from("moved.wav"))
            .is_some()
    );
    assert!(!controller.ui_cache.browser.labels.contains_key(&source.id));
    assert!(!controller.ui_cache.browser.labels.contains_key(&target.id));
    assert_eq!(
        controller.ui.status.text,
        "Moved 1 sample(s) with 1 error(s)"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn apply_source_move_result_reports_cancelled_and_noop_statuses() {
    let temp = tempdir().unwrap();
    let target_root = temp.path().join("source_b");
    std::fs::create_dir_all(&target_root).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let target = SampleSource::new(target_root);
    controller.library.sources.push(target.clone());

    controller
        .drag_drop()
        .apply_source_move_result(SourceMoveResult {
            target_source_id: target.id.clone(),
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: true,
        });
    assert_eq!(controller.ui.status.text, "Move cancelled");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);

    controller
        .drag_drop()
        .apply_source_move_result(SourceMoveResult {
            target_source_id: target.id,
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: false,
        });
    assert_eq!(controller.ui.status.text, "No samples moved");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Warning);
}

#[test]
fn apply_source_move_result_errors_when_target_source_is_missing() {
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);

    controller
        .drag_drop()
        .apply_source_move_result(SourceMoveResult {
            target_source_id: SourceId::from_string("missing"),
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: false,
        });

    assert_eq!(
        controller.ui.status.text,
        "Target source not available for move"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Error);
}
