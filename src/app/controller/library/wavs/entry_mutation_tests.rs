use crate::app::controller::state::audio::LoadedAudio;
use crate::app::controller::state::cache::WavEntriesState;
use crate::app::controller::state::selection::CompareAnchorSample;
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::app::state::CompareAnchorState;
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[test]
fn rewrite_db_entry_for_source_moves_metadata_and_preserves_flags() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("old.wav", Rating::KEEP_1)]);
    let db = controller.database_for(&source).unwrap();
    db.set_tag(Path::new("old.wav"), Rating::KEEP_3).unwrap();
    db.set_looped(Path::new("old.wav"), true).unwrap();
    db.set_last_played_at(Path::new("old.wav"), 42).unwrap();

    controller
        .rewrite_db_entry_for_source(
            &source,
            Path::new("old.wav"),
            Path::new("renamed.wav"),
            512,
            1234,
            Rating::KEEP_3,
        )
        .unwrap();

    assert_eq!(db.index_for_path(Path::new("old.wav")).unwrap(), None);
    assert!(
        db.index_for_path(Path::new("renamed.wav"))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        db.tag_for_path(Path::new("renamed.wav")).unwrap(),
        Some(Rating::KEEP_3)
    );
    assert_eq!(
        db.looped_for_path(Path::new("renamed.wav")).unwrap(),
        Some(true)
    );
    assert_eq!(
        db.last_played_at_for_path(Path::new("renamed.wav"))
            .unwrap(),
        Some(42)
    );
}

#[test]
fn update_selection_paths_rewrites_compare_anchor_and_loaded_state() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("old.wav", Rating::NEUTRAL)]);
    let old_path = Path::new("old.wav");
    let new_path = Path::new("folder/new.wav");
    controller.ui.browser.selection.selected_paths = vec![PathBuf::from("old.wav")];
    controller.sample_view.wav.selected_wav = Some(old_path.to_path_buf());
    controller.sample_view.wav.loaded_wav = Some(old_path.to_path_buf());
    controller.set_ui_loaded_wav(Some(old_path.to_path_buf()));
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: old_path.to_path_buf(),
        bytes: Arc::<[u8]>::from(vec![0_u8].into_boxed_slice()),
        duration_seconds: 1.0,
        sample_rate: 44_100,
    });
    controller.sample_view.wav.compare_anchor = Some(CompareAnchorSample {
        source_id: source.id.clone(),
        relative_path: old_path.to_path_buf(),
    });
    controller.ui.compare_anchor = Some(CompareAnchorState {
        source_id: source.id.clone(),
        relative_path: old_path.to_path_buf(),
        label: String::from("old.wav"),
    });
    controller.ui.waveform.compare_anchor_label = Some(String::from("old.wav"));

    controller.update_selection_paths(&source, old_path, new_path);

    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![new_path.to_path_buf()]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(new_path)
    );
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(new_path)
    );
    assert_eq!(controller.ui.loaded_wav.as_deref(), Some(new_path));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.relative_path.as_path()),
        Some(new_path)
    );
    assert_eq!(
        controller
            .sample_view
            .wav
            .compare_anchor
            .as_ref()
            .map(|anchor| anchor.relative_path.as_path()),
        Some(new_path)
    );
    assert_eq!(
        controller
            .ui
            .compare_anchor
            .as_ref()
            .map(|anchor| anchor.relative_path.as_path()),
        Some(new_path)
    );
    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("new")
    );
}

#[test]
fn update_cached_entry_rewrites_cache_lookup_db_and_focus_path_on_rename() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("old.wav", Rating::NEUTRAL)]);
    let db = controller.database_for(&source).unwrap();
    db.set_looped(Path::new("old.wav"), true).unwrap();
    db.set_last_played_at(Path::new("old.wav"), 77).unwrap();
    controller
        .ui_cache
        .browser
        .labels
        .insert(source.id.clone(), vec![String::from("old")]);

    let mut cache = WavEntriesState::new(1, 50);
    cache.insert_page(0, vec![sample_entry("old.wav", Rating::NEUTRAL)]);
    controller
        .cache
        .wav
        .entries
        .insert(source.id.clone(), cache);
    controller.ui.browser.selection.selected_paths = vec![PathBuf::from("old.wav")];
    controller.ui.browser.selection.last_focused_index = Some(0);
    controller.ui.browser.selection.last_focused_path = Some(PathBuf::from("old.wav"));
    controller.sample_view.wav.compare_anchor = Some(CompareAnchorSample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("old.wav"),
    });
    controller.ui.compare_anchor = Some(CompareAnchorState {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("old.wav"),
        label: String::from("old.wav"),
    });
    controller.ui.waveform.compare_anchor_label = Some(String::from("old.wav"));

    let mut updated = sample_entry("renamed.wav", Rating::KEEP_1);
    updated.file_size = 88;
    updated.modified_ns = 99;
    controller.update_cached_entry(&source, Path::new("old.wav"), updated);

    assert!(
        controller
            .wav_index_for_path(Path::new("old.wav"))
            .is_none()
    );
    assert!(
        controller
            .wav_index_for_path(Path::new("renamed.wav"))
            .is_some()
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("renamed.wav")]
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("renamed.wav"))
    );
    assert_eq!(
        controller
            .cache
            .wav
            .entries
            .get(&source.id)
            .and_then(|cache| cache.lookup.get(Path::new("renamed.wav"))),
        Some(&0)
    );
    assert_eq!(
        controller
            .cache
            .wav
            .entries
            .get(&source.id)
            .and_then(|cache| cache.lookup.get(Path::new("old.wav"))),
        None
    );
    assert_eq!(db.index_for_path(Path::new("old.wav")).unwrap(), None);
    assert!(
        db.index_for_path(Path::new("renamed.wav"))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        db.tag_for_path(Path::new("renamed.wav")).unwrap(),
        Some(Rating::KEEP_1)
    );
    assert_eq!(
        db.looped_for_path(Path::new("renamed.wav")).unwrap(),
        Some(true)
    );
    assert_eq!(
        db.last_played_at_for_path(Path::new("renamed.wav"))
            .unwrap(),
        Some(77)
    );
    assert_eq!(
        controller
            .ui
            .compare_anchor
            .as_ref()
            .map(|anchor| anchor.relative_path.as_path()),
        Some(Path::new("renamed.wav"))
    );
    assert_eq!(
        controller.ui_cache.browser.labels.get(&source.id),
        Some(&vec![String::from("renamed")])
    );
}

#[test]
fn insert_cached_entry_preserves_existing_browser_labels_when_index_is_known() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    controller.ui_cache.browser.labels.insert(
        source.id.clone(),
        vec![String::from("a"), String::from("c")],
    );
    let db = controller.database_for(&source).unwrap();
    db.upsert_file(Path::new("b.wav"), 0, 0)
        .expect("insert middle db row");

    controller.insert_cached_entry(&source, sample_entry("b.wav", Rating::KEEP_1));

    assert_eq!(
        controller.ui_cache.browser.labels.get(&source.id),
        Some(&vec![String::from("a"), String::new(), String::from("c"),])
    );
}
