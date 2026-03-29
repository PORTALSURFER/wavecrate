use super::super::test_support::{dummy_controller, sample_entry};
use super::super::*;
use crate::sample_sources::SourceDatabase;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn selecting_legacy_missing_sample_prunes_it_and_sets_waveform_notice() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.set_wav_entries_for_tests(vec![WavEntry {
        relative_path: PathBuf::from("one.wav"),
        file_size: 1,
        modified_ns: 1,
        content_hash: Some(String::from("hash-one")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: true,
        last_played_at: None,
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_wav_by_path(Path::new("one.wav"));

    assert!(
        controller
            .ui
            .waveform
            .notice
            .as_ref()
            .is_some_and(|msg| msg.contains("one.wav"))
    );
    assert!(controller.sample_view.wav.loaded_audio.is_none());
    assert!(
        controller
            .wav_index_for_path(Path::new("one.wav"))
            .is_none()
    );
    let db = controller.database_for(&source).unwrap();
    assert!(db.entry_for_path(Path::new("one.wav")).unwrap().is_none());
}

#[test]
fn read_failure_prunes_sample_row() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let rel = PathBuf::from("gone.wav");
    controller.set_wav_entries_for_tests(vec![WavEntry {
        relative_path: rel.clone(),
        file_size: 1,
        modified_ns: 1,
        content_hash: Some(String::from("hash-gone")),
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let err = controller
        .load_waveform_for_selection(&source, &rel)
        .unwrap_err();
    assert!(err.contains("Failed to read"));
    assert!(controller.wav_index_for_path(&rel).is_none());
    assert_eq!(controller.visible_browser_len(), 0);
    let db = controller.database_for(&source).unwrap();
    assert!(db.entry_for_path(&rel).unwrap().is_none());
}

#[test]
fn apply_wav_entries_clears_file_missing_lookup_for_present_sources() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller
        .ui_cache
        .browser
        .analysis_failures
        .insert(source.id.clone(), HashMap::new());
    controller.cache_db(&source).unwrap();
    controller.library.missing.wavs.insert(
        source.id.clone(),
        [PathBuf::from("gone.wav")].into_iter().collect(),
    );

    controller.apply_wav_entries_with_params(
        crate::app::controller::ui::loading::ApplyWavEntriesParams {
            entries: vec![sample_entry(
                "alive.wav",
                crate::sample_sources::Rating::NEUTRAL,
            )],
            total: 1,
            page_size: controller.wav_entries.page_size,
            page_index: 0,
            from_cache: true,
            source_id: Some(source.id.clone()),
            elapsed: None,
        },
    );

    assert!(!controller.library.missing.wavs.contains_key(&source.id));
}

#[test]
fn prune_missing_sample_removes_cache_and_db_entry_when_inactive() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    let other_root = temp.path().join("other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&other_root).unwrap();
    let renderer = WaveformRenderer::new(10, 10);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    let other = SampleSource::new(other_root);
    controller.library.sources.push(source.clone());
    controller.library.sources.push(other.clone());
    controller.selection_state.ctx.selected_source = Some(other.id.clone());
    controller.cache_db(&source).unwrap();

    let db = SourceDatabase::open(&root).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(
            Path::new("one.wav"),
            1,
            1,
            "hash-one",
            crate::sample_sources::Rating::KEEP_1,
            false,
        )
        .unwrap();
    batch.commit().unwrap();
    let mut cache = WavEntriesState::new(1, controller.wav_entries.page_size);
    cache.insert_page(
        0,
        vec![WavEntry {
            relative_path: PathBuf::from("one.wav"),
            file_size: 1,
            modified_ns: 1,
            content_hash: Some(String::from("hash-one")),
            tag: crate::sample_sources::Rating::KEEP_1,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        }],
    );
    controller
        .cache
        .wav
        .entries
        .insert(source.id.clone(), cache);

    assert!(
        controller
            .prune_missing_sample(&source, Path::new("one.wav"))
            .unwrap()
    );

    assert!(db.entry_for_path(Path::new("one.wav")).unwrap().is_none());
    assert!(
        controller
            .cache
            .wav
            .entries
            .get(&source.id)
            .is_some_and(|entries| entries.pages.is_empty())
    );
    let pending = db.list_pending_renames().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].relative_path, PathBuf::from("one.wav"));
    assert_eq!(pending[0].tag, crate::sample_sources::Rating::KEEP_1);
}

#[test]
fn prune_missing_sample_removes_db_entry_without_cache() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    let other_root = temp.path().join("other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&other_root).unwrap();
    let renderer = WaveformRenderer::new(10, 10);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    let other = SampleSource::new(other_root);
    controller.library.sources.push(source.clone());
    controller.library.sources.push(other.clone());
    controller.selection_state.ctx.selected_source = Some(other.id.clone());
    controller.cache_db(&source).unwrap();

    let db = SourceDatabase::open(&root).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(
            Path::new("ghost.wav"),
            1,
            1,
            "hash-ghost",
            crate::sample_sources::Rating::NEUTRAL,
            false,
        )
        .unwrap();
    batch.commit().unwrap();

    assert!(
        controller
            .prune_missing_sample(&source, Path::new("ghost.wav"))
            .unwrap()
    );

    assert!(db.entry_for_path(Path::new("ghost.wav")).unwrap().is_none());
    assert!(!controller.cache.wav.entries.contains_key(&source.id));
    assert!(controller.sample_missing(&source.id, Path::new("ghost.wav")));
}
