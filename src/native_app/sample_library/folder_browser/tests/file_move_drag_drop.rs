use super::super::FileEntry;
use super::*;
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery;
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn seed_file_collections(
    db: &SourceDatabase,
    relative_path: &str,
    collections: &[SampleCollection],
) {
    let path = Path::new(relative_path);
    db.upsert_file(path, 8, 1).expect("upsert source row");
    let mut batch = db.write_batch().expect("open write batch");
    for collection in collections {
        batch
            .add_collection(path, *collection)
            .expect("add collection membership");
    }
    batch.commit().expect("commit source metadata");
}

#[test]
fn file_drag_drop_moves_selected_files_into_target_folder() {
    let root = temp_source_root("wavecrate-gui-file-drag-drop");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let hat = drums.join("hat.wav");
    for file in [&kick, &snare, &hat] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));
    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    let moved_kick = loops.join("kick.wav");
    let moved_snare = loops.join("snare.wav");
    assert_eq!(result.moved_paths.len(), 2);
    assert!(!kick.exists());
    assert!(!snare.exists());
    assert!(hat.is_file());
    assert!(moved_kick.is_file());
    assert!(moved_snare.is_file());
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(
        browser.selection.selected_folder_ids,
        HashSet::from([path_id(&drums)])
    );
    assert_eq!(browser.selected_file_paths(), vec![hat.clone()]);
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["hat.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_restores_single_source_folder_and_next_visible_sample() {
    let root = temp_source_root("wavecrate-gui-file-drag-restore-source");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let files = (0..72)
        .map(|index| drums.join(format!("sample_{index:03}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 50.0 * 22.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 72,
            viewport_start: 50,
            viewport_end: 68,
            window_start: 46,
            window_end: 72,
        },
    });
    browser.select_file(path_id(&files[60]));
    browser
        .selection
        .selected_folder_ids
        .insert(path_id(&loops));
    browser.selection.selected_folder_ids_explicit = true;

    browser.begin_file_drag(path_id(&files[60]), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    assert_eq!(
        result.moved_paths,
        vec![(files[60].clone(), loops.join("sample_060.wav"))]
    );
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(
        browser.selection.selected_folder_ids,
        HashSet::from([path_id(&drums)])
    );
    assert!(!browser.selection.selected_folder_ids_explicit);
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&files[61]).as_str())
    );
    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert_eq!(visible.total_count, 71);
    assert_eq!(visible.rows.len(), visible.window.window_len());
    assert!(
        visible
            .rows
            .iter()
            .all(|row| row.file.id != path_id(&files[60])),
        "moved file should not remain materialized in the source list"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_remaps_active_similarity_scores_to_moved_path() {
    let root = temp_source_root("wavecrate-gui-file-drag-similarity-remap");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let anchor = drums.join("anchor.wav");
    let near = drums.join("near.wav");
    fs::write(&anchor, [0_u8; 8]).expect("write anchor");
    fs::write(&near, [0_u8; 8]).expect("write near");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    let anchor_id = path_id(&anchor);
    let near_id = path_id(&near);
    let moved_near_id = path_id(&loops.join("near.wav"));
    browser.set_similarity_scores_for_tests(
        anchor_id.clone(),
        [(near_id.clone(), 0.75)].into_iter().collect(),
    );
    browser.select_file(near_id.clone());

    browser.begin_file_drag(near_id.clone(), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    assert_eq!(browser.similarity_anchor_id(), Some(anchor_id.as_str()));
    assert_eq!(browser.similarity_display_strength_for_file(&near_id), None);
    assert!(
        browser
            .similarity_display_strength_for_file(&moved_near_id)
            .is_some(),
        "moved candidate should retain its similarity score under the new path"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn collection_file_drag_drop_moves_file_to_folder_and_preserves_collection_membership() {
    let root = temp_source_root("wavecrate-gui-collection-file-drag-drop");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(&snare, [0_u8; 8]).expect("write snare");
    let first = SampleCollection::new(0).expect("first collection");
    let second = SampleCollection::new(1).expect("second collection");
    let db = SourceDatabase::open(&root).expect("open source database");
    seed_file_collections(&db, "drums/kick.wav", &[first, second]);
    seed_file_collections(&db, "drums/snare.wav", &[first]);

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_collection(first);
    browser.select_file(path_id(&kick));
    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    let moved_kick = loops.join("kick.wav");
    assert_eq!(result.moved_paths, vec![(kick.clone(), moved_kick.clone())]);
    assert!(!kick.exists());
    assert!(moved_kick.is_file());
    assert!(snare.is_file());
    assert_eq!(
        db.collections_for_path(Path::new("drums/kick.wav"))
            .expect("old collections"),
        Vec::<SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("loops/kick.wav"))
            .expect("moved collections"),
        vec![first, second]
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "snare.wav"]
    );
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&moved_kick).as_str())
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn collection_multi_file_move_replaces_stale_missing_rows_with_new_paths() {
    let root = temp_source_root("wavecrate-gui-collection-multi-file-move");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let hat = drums.join("hat.wav");
    for file in [&kick, &snare, &hat] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let collection = SampleCollection::new(0).expect("collection");
    let db = SourceDatabase::open(&root).expect("open source database");
    seed_file_collections(&db, "drums/kick.wav", &[collection]);
    seed_file_collections(&db, "drums/snare.wav", &[collection]);
    seed_file_collections(&db, "drums/hat.wav", &[collection]);

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_collection(collection);
    browser.select_file(path_id(&kick));
    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    browser.source.sources[0]
        .missing_collection_snapshot
        .add_missing_file(FileEntry::missing_collection_member(
            &kick,
            Rating::NEUTRAL,
            false,
            vec![collection],
            None,
            None,
        ));
    browser.source.sources[0]
        .missing_collection_snapshot
        .add_missing_file(FileEntry::missing_collection_member(
            &snare,
            Rating::NEUTRAL,
            false,
            vec![collection],
            None,
            None,
        ));
    browser.refresh_missing_collection_state();

    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    let moved_kick = loops.join("kick.wav");
    let moved_snare = loops.join("snare.wav");
    assert_eq!(
        result.moved_paths,
        vec![
            (kick.clone(), moved_kick.clone()),
            (snare.clone(), moved_snare.clone())
        ]
    );
    assert_eq!(
        db.collections_for_path(Path::new("drums/kick.wav"))
            .expect("old kick collections"),
        Vec::<SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("drums/snare.wav"))
            .expect("old snare collections"),
        Vec::<SampleCollection>::new()
    );
    assert_eq!(
        db.collections_for_path(Path::new("loops/kick.wav"))
            .expect("moved kick collections"),
        vec![collection]
    );
    assert_eq!(
        db.collections_for_path(Path::new("loops/snare.wav"))
            .expect("moved snare collections"),
        vec![collection]
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.id.as_str())
            .collect::<Vec<_>>(),
        vec![path_id(&hat), path_id(&moved_kick), path_id(&moved_snare)]
    );
    assert!(
        browser
            .selected_audio_files()
            .iter()
            .all(|file| !file.is_missing()),
        "moved collection files should not leave stale broken rows behind"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn collection_file_move_conflict_rename_preserves_collection_on_moved_destination() {
    let root = temp_source_root("wavecrate-gui-collection-file-conflict-rename");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let existing = loops.join("kick.wav");
    fs::write(&kick, b"source").expect("write kick");
    fs::write(&existing, b"existing").expect("write existing");
    let collection = SampleCollection::new(0).expect("collection");
    let db = SourceDatabase::open(&root).expect("open source database");
    seed_file_collections(&db, "drums/kick.wav", &[collection]);
    db.upsert_file(Path::new("loops/kick.wav"), 8, 1)
        .expect("upsert existing destination");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_collection(collection);
    browser.select_file(path_id(&kick));
    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should queue conflict");
    assert_eq!(browser.pending_file_move_conflict_count(), 1);

    let result = submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Rename)
        .expect("rename conflict should move");

    let renamed = loops.join("kick_copy001.wav");
    assert_eq!(result.moved_paths, vec![(kick.clone(), renamed.clone())]);
    assert!(!kick.exists());
    assert_eq!(fs::read(&existing).expect("read existing"), b"existing");
    assert_eq!(fs::read(&renamed).expect("read renamed"), b"source");
    assert_eq!(
        db.collections_for_path(Path::new("loops/kick_copy001.wav"))
            .expect("renamed collections"),
        vec![collection]
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick_copy001.wav"],
        "moved file should remain in the active collection view"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_moves_selected_files_outside_current_view_window() {
    let root = temp_source_root("wavecrate-gui-file-drag-offscreen-selection");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let files = (0..80)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&files[4]));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 40.0 * 22.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 80,
            viewport_start: 40,
            viewport_end: 58,
            window_start: 36,
            window_end: 62,
        },
    });
    browser.select_file_with_modifiers(
        path_id(&files[44]),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    browser.select_file_with_modifiers(
        path_id(&files[55]),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );

    browser.begin_file_drag(path_id(&files[55]), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    assert_eq!(result.moved_paths.len(), 3);
    for index in [4, 44, 55] {
        assert!(!files[index].exists(), "source should move: {index}");
        assert!(
            loops.join(format!("sample_{index:02}.wav")).is_file(),
            "destination should exist: {index}"
        );
    }
    assert!(files[5].is_file());
    assert!(files[54].is_file());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_clamps_bottom_view_after_moving_files_out() {
    let root = temp_source_root("wavecrate-gui-file-drag-bottom-clamp");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let files = (0..100)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 80.0 * 22.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 100,
            viewport_start: 80,
            viewport_end: 98,
            window_start: 76,
            window_end: 100,
        },
    });
    browser.select_file(path_id(&files[80]));
    for file in files.iter().skip(81) {
        browser.select_file_with_modifiers(
            path_id(file),
            PointerModifiers {
                command: true,
                ..Default::default()
            },
        );
    }

    browser.begin_file_drag(path_id(&files[80]), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    assert_eq!(result.moved_paths.len(), 20);
    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert_eq!(visible.total_count, 80);
    assert_eq!(visible.window.viewport_start, 62);
    assert_eq!(visible.window.viewport_end, 80);
    assert_eq!(visible.window.window_start, 58);
    assert_eq!(visible.window.window_end, 80);
    assert_eq!(visible.rows.len(), visible.window.window_len());
    assert_eq!(
        visible.rows.last().map(|row| row.file.id.as_str()),
        Some(path_id(&files[79]).as_str())
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_moves_shift_selected_offscreen_range() {
    let root = temp_source_root("wavecrate-gui-file-drag-offscreen-shift-range");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let files = (0..80)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&files[4]));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 40.0 * 22.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 80,
            viewport_start: 40,
            viewport_end: 58,
            window_start: 36,
            window_end: 62,
        },
    });
    browser.select_file_with_modifiers(
        path_id(&files[55]),
        PointerModifiers {
            shift: true,
            ..Default::default()
        },
    );

    browser.begin_file_drag(path_id(&files[55]), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    assert_eq!(result.moved_paths.len(), 52);
    for (index, file) in files.iter().enumerate().take(56).skip(4) {
        assert!(!file.exists(), "source should move: {index}");
        assert!(
            loops.join(format!("sample_{index:02}.wav")).is_file(),
            "destination should exist: {index}"
        );
    }
    assert!(files[3].is_file());
    assert!(files[56].is_file());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_moves_explicit_selection_after_keyboard_focus_navigation() {
    let root = temp_source_root("wavecrate-gui-file-drag-explicit-focus-navigation");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let tom = drums.join("tom.wav");
    for file in [&hat, &kick, &snare, &tom] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&hat));
    browser
        .toggle_focused_sample_selection(&Default::default())
        .expect("mark first file");
    browser.navigate_vertical(1, false);
    browser.navigate_vertical(1, false);
    browser
        .toggle_focused_sample_selection(&Default::default())
        .expect("mark third file");
    browser.navigate_vertical(1, false);
    assert_eq!(browser.selected_file_id(), Some(path_id(&tom).as_str()));
    assert_eq!(
        browser.selected_file_paths(),
        vec![hat.clone(), snare.clone()]
    );

    browser.begin_file_drag(path_id(&tom), Point::new(4.0, 8.0));
    let result =
        submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    assert_eq!(
        result.moved_paths,
        vec![
            (hat.clone(), loops.join("hat.wav")),
            (snare.clone(), loops.join("snare.wav"))
        ]
    );
    assert!(!hat.exists());
    assert!(kick.is_file());
    assert!(!snare.exists());
    assert!(tom.is_file());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_drag_drop_preserves_rating_metadata_after_move() {
    let root = temp_source_root("wavecrate-gui-file-drag-rating");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write wav");

    let db = SourceDatabase::open(&root).expect("open source db");
    db.upsert_file(std::path::Path::new("drums/kick.wav"), 8, 1)
        .expect("upsert kick");
    db.set_tag(std::path::Path::new("drums/kick.wav"), Rating::new(2))
        .expect("set rating");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));
    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));

    submit_folder_drop(&mut browser, &path_id(&loops)).expect("file drag/drop should move");

    let moved_kick = loops.join("kick.wav");
    browser.activate_folder(path_id(&loops));
    let moved = browser
        .selected_audio_files()
        .into_iter()
        .find(|file| file.id == path_id(&moved_kick))
        .expect("moved kick row");
    assert_eq!(moved.rating, Rating::new(2));
    assert_eq!(
        db.tag_for_path(std::path::Path::new("drums/kick.wav"))
            .expect("read old rating"),
        None
    );
    assert_eq!(
        db.tag_for_path(std::path::Path::new("loops/kick.wav"))
            .expect("read moved rating"),
        Some(Rating::new(2))
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn duplicate_same_preserves_source_metadata_without_removing_original() {
    let root = temp_source_root("wavecrate-gui-duplicate-same-metadata");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create source folder");
    let kick = drums.join("kick.wav");
    let duplicate = drums.join("kick_copy001.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");

    let first_collection = SampleCollection::new(0).expect("first collection");
    let second_collection = SampleCollection::new(1).expect("second collection");
    let source_db = SourceDatabase::open(&root).expect("open source db");
    let source_relative = Path::new("drums/kick.wav");
    let duplicate_relative = Path::new("drums/kick_copy001.wav");
    let mut batch = source_db.write_batch().expect("open metadata batch");
    batch
        .upsert_file_with_hash(source_relative, 8, 1, "kick-content-hash")
        .expect("upsert source row");
    batch
        .set_tag(source_relative, Rating::new(2))
        .expect("set rating");
    batch
        .set_looped(source_relative, true)
        .expect("set loop marker");
    batch
        .set_locked(source_relative, true)
        .expect("set keep lock");
    batch
        .set_sound_type(
            source_relative,
            Some(wavecrate::sample_sources::SampleSoundType::Kick),
        )
        .expect("set sound type");
    batch
        .set_user_tag(source_relative, Some("Punchy"))
        .expect("set user tag");
    batch
        .set_tag_named(source_relative, true)
        .expect("set tag-named marker");
    batch
        .set_last_played_at(source_relative, 1234)
        .expect("set last played");
    batch
        .replace_tags_for_path(
            source_relative,
            &[String::from("Hard"), String::from("Drum")],
        )
        .expect("set normal tags");
    batch
        .add_collection(source_relative, first_collection)
        .expect("set first collection");
    batch
        .add_collection(source_relative, second_collection)
        .expect("set second collection");
    batch
        .set_last_curated_at(source_relative, 5678)
        .expect("restore curation timestamp");
    batch.commit().expect("commit source metadata");

    fs::copy(&kick, &duplicate).expect("duplicate sample");
    wavecrate::sample_sources::persist_copied_file_metadata(&root, &root, &kick, &duplicate)
        .expect("copy metadata");

    assert_eq!(
        source_db
            .tag_for_path(source_relative)
            .expect("read original rating"),
        Some(Rating::new(2))
    );
    assert_eq!(
        source_db
            .tag_for_path(duplicate_relative)
            .expect("read duplicate rating"),
        Some(Rating::new(2))
    );
    assert_eq!(
        source_db
            .looped_for_path(duplicate_relative)
            .expect("read loop marker"),
        Some(true)
    );
    assert_eq!(
        source_db
            .locked_for_path(duplicate_relative)
            .expect("read keep lock"),
        Some(true)
    );
    assert_eq!(
        source_db
            .tag_labels_for_path(duplicate_relative)
            .expect("read duplicate tags"),
        vec![String::from("Drum"), String::from("Hard")]
    );
    assert_eq!(
        source_db
            .collections_for_path(duplicate_relative)
            .expect("read duplicate collections"),
        vec![first_collection, second_collection]
    );
    assert_eq!(
        source_db
            .last_curated_at_for_path(duplicate_relative)
            .expect("read duplicate curation timestamp"),
        Some(5678)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cut_paste_moves_files_between_sources_and_preserves_metadata() {
    let source_root = temp_source_root("wavecrate-gui-cut-paste-source-a");
    let target_root = temp_source_root("wavecrate-gui-cut-paste-source-b");
    let drums = source_root.join("drums");
    let loops = target_root.join("loops");
    fs::create_dir_all(&drums).expect("create source folder");
    fs::create_dir_all(&loops).expect("create target folder");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");

    let first_collection = SampleCollection::new(0).expect("first collection");
    let second_collection = SampleCollection::new(1).expect("second collection");
    let source_db = SourceDatabase::open(&source_root).expect("open source db");
    let source_relative = Path::new("drums/kick.wav");
    let mut batch = source_db.write_batch().expect("open metadata batch");
    batch
        .upsert_file_with_hash(source_relative, 8, 1, "kick-content-hash")
        .expect("upsert source row");
    batch
        .set_tag(source_relative, Rating::new(2))
        .expect("set rating");
    batch
        .set_looped(source_relative, true)
        .expect("set loop marker");
    batch
        .set_locked(source_relative, true)
        .expect("set keep lock");
    batch
        .set_sound_type(
            source_relative,
            Some(wavecrate::sample_sources::SampleSoundType::Kick),
        )
        .expect("set sound type");
    batch
        .set_user_tag(source_relative, Some("Punchy"))
        .expect("set user tag");
    batch
        .set_tag_named(source_relative, true)
        .expect("set tag-named marker");
    batch
        .set_last_played_at(source_relative, 1234)
        .expect("set last played");
    batch
        .replace_tags_for_path(
            source_relative,
            &[String::from("Hard"), String::from("Drum")],
        )
        .expect("set normal tags");
    batch
        .add_collection(source_relative, first_collection)
        .expect("set first collection");
    batch
        .add_collection(source_relative, second_collection)
        .expect("set second collection");
    batch
        .set_last_curated_at(source_relative, 5678)
        .expect("restore curation timestamp");
    batch.commit().expect("commit source metadata");

    let sources = vec![
        wavecrate::sample_sources::SampleSource::new_with_id(
            wavecrate::sample_sources::SourceId::from_string("source-a"),
            source_root.clone(),
        ),
        wavecrate::sample_sources::SampleSource::new_with_id(
            wavecrate::sample_sources::SourceId::from_string("source-b"),
            target_root.clone(),
        ),
    ];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);
    load_source_for_test(&mut browser, "source-b", 11);
    let loops_id = path_id(&loops);
    browser.activate_folder(loops_id.clone());

    let moved = loops.join("kick.wav");
    let moved_id = path_id(&moved);
    let result = submit_cut_paste(&mut browser, &[kick.display().to_string()], &loops_id)
        .expect("cut paste should move across sources");

    assert_eq!(result.moved_paths, vec![(kick.clone(), moved.clone())]);
    assert!(!kick.exists());
    assert!(moved.is_file());
    assert_eq!(browser.selected_source_id(), "source-b");
    assert_eq!(browser.selected_folder_id(), Some(loops_id.as_str()));
    assert_eq!(browser.selected_file_id(), Some(moved_id.as_str()));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| (file.id.as_str(), file.rating, file.rating_locked))
            .collect::<Vec<_>>(),
        vec![(moved_id.as_str(), Rating::new(2), true)]
    );

    let target_db = SourceDatabase::open(&target_root).expect("open target db");
    let target_relative = Path::new("loops/kick.wav");
    assert_eq!(
        source_db
            .tag_for_path(source_relative)
            .expect("old source row should be gone"),
        None
    );
    let target_entry = target_db
        .entry_for_path(target_relative)
        .expect("read target row")
        .expect("target row");
    assert_eq!(
        target_entry.content_hash.as_deref(),
        Some("kick-content-hash")
    );
    assert_eq!(target_entry.tag, Rating::new(2));
    assert!(target_entry.looped);
    assert!(target_entry.locked);
    assert_eq!(
        target_entry.sound_type,
        Some(wavecrate::sample_sources::SampleSoundType::Kick)
    );
    assert_eq!(target_entry.user_tag.as_deref(), Some("Punchy"));
    assert!(target_entry.tag_named);
    assert_eq!(target_entry.last_played_at, Some(1234));
    assert_eq!(target_entry.last_curated_at, Some(5678));
    assert_eq!(
        target_db
            .tag_labels_for_path(target_relative)
            .expect("target normal tags"),
        vec![String::from("Drum"), String::from("Hard")]
    );
    assert_eq!(
        target_db
            .collections_for_path(target_relative)
            .expect("target collections"),
        vec![first_collection, second_collection]
    );

    load_source_for_test(&mut browser, "source-a", 12);
    browser.activate_folder(path_id(&drums));
    assert!(
        browser.selected_audio_files().is_empty(),
        "source cache should no longer show the moved file"
    );
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(target_root);
}

#[test]
fn cut_paste_from_protected_source_copies_to_writable_target_and_keeps_source_metadata() {
    let source_root = temp_source_root("wavecrate-gui-protected-cut-paste-source");
    let target_root = temp_source_root("wavecrate-gui-protected-cut-paste-target");
    let drums = source_root.join("drums");
    let inbox = target_root.join("_Wavecrate Inbox");
    fs::create_dir_all(&drums).expect("create protected source folder");
    fs::create_dir_all(&inbox).expect("create primary inbox");
    let kick = drums.join("kick.wav");
    fs::write(&kick, [0_u8; 8]).expect("write kick");

    let protected_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::new(),
        source_root.clone(),
    )
    .protected();
    let primary_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::new(),
        target_root.clone(),
    )
    .primary();
    let protected_db_root = protected_source
        .database_root()
        .expect("protected metadata root");
    let protected_db = SourceDatabase::open_for_user_metadata_write_with_database_root(
        &protected_source.root,
        &protected_db_root,
    )
    .expect("open protected external db");
    let source_relative = Path::new("drums/kick.wav");
    let mut protected_batch = protected_db
        .write_batch()
        .expect("protected metadata batch");
    protected_batch
        .upsert_file_with_hash(source_relative, 8, 1, "protected-kick-hash")
        .expect("upsert protected metadata");
    protected_batch
        .set_tag(source_relative, Rating::new(3))
        .expect("set protected rating");
    protected_batch
        .add_collection(
            source_relative,
            SampleCollection::new(0).expect("collection"),
        )
        .expect("set protected collection");
    protected_batch.commit().expect("commit protected metadata");

    let sources = vec![protected_source.clone(), primary_source.clone()];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);
    load_source_for_test(&mut browser, primary_source.id.as_str(), 21);
    let inbox_id = path_id(&inbox);
    browser.activate_folder(inbox_id.clone());

    let copied = inbox.join("kick.wav");
    let copied_id = path_id(&copied);
    let result = submit_cut_paste(&mut browser, &[kick.display().to_string()], &inbox_id)
        .expect("protected cut paste should copy into writable target");

    assert_eq!(result.moved_paths, vec![(kick.clone(), copied.clone())]);
    assert!(kick.is_file(), "protected source file should remain");
    assert!(copied.is_file(), "copy should be written into target");
    assert_eq!(browser.selected_file_id(), Some(copied_id.as_str()));
    assert_eq!(
        protected_db
            .tag_for_path(source_relative)
            .expect("protected metadata remains"),
        Some(Rating::new(3))
    );

    let target_db = SourceDatabase::open(&target_root).expect("open target db");
    let target_relative = Path::new("_Wavecrate Inbox/kick.wav");
    let target_entry = target_db
        .entry_for_path(target_relative)
        .expect("read target metadata")
        .expect("target metadata row");
    assert_eq!(
        target_entry.content_hash.as_deref(),
        Some("protected-kick-hash")
    );
    assert_eq!(target_entry.tag, Rating::new(3));
    assert_eq!(
        target_db
            .collections_for_path(target_relative)
            .expect("target collection copied"),
        vec![SampleCollection::new(0).expect("collection")]
    );

    load_source_for_test(&mut browser, protected_source.id.as_str(), 22);
    browser.activate_folder(path_id(&drums));
    let original_id = path_id(&kick);
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.id.as_str())
            .collect::<Vec<_>>(),
        vec![original_id.as_str()]
    );
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(target_root);
    let _ = fs::remove_dir_all(protected_db_root);
}

#[test]
fn file_drag_from_protected_source_copies_to_writable_source_root() {
    let source_root = temp_source_root("wavecrate-gui-protected-drag-source");
    let target_root = temp_source_root("wavecrate-gui-protected-drag-target");
    let drums = source_root.join("drums");
    fs::create_dir_all(&drums).expect("create protected source folder");
    fs::create_dir_all(&target_root).expect("create target source root");
    let kick = drums.join("kick.wav");
    fs::write(&kick, b"kick").expect("write kick");

    let protected_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("protected-drag-source"),
        source_root.clone(),
    )
    .protected();
    let target_source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("target-drag-source"),
        target_root.clone(),
    );
    let mut browser =
        FolderBrowserState::from_sample_sources(&[protected_source.clone(), target_source.clone()]);
    browser.activate_folder(path_id(&drums));

    assert!(browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0)));
    assert!(
        browser.can_drop_drag_on_source(target_source.id.as_str()),
        "writable source root should accept protected-origin file drags"
    );
    let result = submit_source_drop(&mut browser, target_source.id.as_str())
        .expect("protected file drag should copy into writable source root");

    let copied = target_root.join("kick.wav");
    assert_eq!(result.moved_paths, vec![(kick.clone(), copied.clone())]);
    assert!(kick.is_file(), "protected source original should remain");
    assert!(
        copied.is_file(),
        "drop onto source should land at source root"
    );
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(target_root);
}

#[test]
fn file_drag_into_protected_source_root_is_blocked() {
    let source_root = temp_source_root("wavecrate-gui-drag-source");
    let protected_root = temp_source_root("wavecrate-gui-drag-protected-target");
    let drums = source_root.join("drums");
    fs::create_dir_all(&drums).expect("create source folder");
    fs::create_dir_all(&protected_root).expect("create protected root");
    let kick = drums.join("kick.wav");
    fs::write(&kick, b"kick").expect("write kick");

    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("normal-drag-source"),
        source_root.clone(),
    );
    let protected = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("protected-drag-target"),
        protected_root.clone(),
    )
    .protected();
    let mut browser = FolderBrowserState::from_sample_sources(&[source.clone(), protected.clone()]);
    browser.activate_folder(path_id(&drums));

    assert!(browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0)));
    assert!(
        !browser.can_drop_drag_on_source(protected.id.as_str()),
        "protected source roots must not become valid drop candidates"
    );
    let error = browser
        .drop_drag_on_source(protected.id.as_str())
        .expect_err("drop into protected source root should be blocked");

    assert_eq!(
        error,
        crate::native_app::protected_source_feedback::PROTECTED_SOURCE_BLOCKED_STATUS
    );
    assert!(
        kick.is_file(),
        "source file should remain after blocked drop"
    );
    assert!(
        !protected_root.join("kick.wav").exists(),
        "blocked drop should not write into protected source"
    );
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(protected_root);
}

#[test]
fn cut_paste_into_protected_source_adds_new_files_but_blocks_overwrite() {
    let source_root = temp_source_root("wavecrate-gui-protected-target-source");
    let protected_root = temp_source_root("wavecrate-gui-protected-target");
    let drums = source_root.join("drums");
    let inbox = protected_root.join("incoming");
    fs::create_dir_all(&drums).expect("create source folder");
    fs::create_dir_all(&inbox).expect("create protected inbox");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let protected_snare = inbox.join("snare.wav");
    fs::write(&kick, b"kick").expect("write kick");
    fs::write(&snare, b"snare").expect("write snare");
    fs::write(&protected_snare, b"protected snare").expect("write existing protected file");

    let source = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("normal-source"),
        source_root.clone(),
    );
    let protected = wavecrate::sample_sources::SampleSource::new_with_id(
        wavecrate::sample_sources::SourceId::from_string("protected-target"),
        protected_root.clone(),
    )
    .protected();
    let protected_db_root = protected.database_root().expect("protected metadata root");
    let sources = vec![source, protected.clone()];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);
    load_source_for_test(&mut browser, protected.id.as_str(), 23);
    let inbox_id = path_id(&inbox);
    browser.activate_folder(inbox_id.clone());

    let moved_kick = inbox.join("kick.wav");
    let result = submit_cut_paste(
        &mut browser,
        &[kick.display().to_string(), snare.display().to_string()],
        &inbox_id,
    )
    .expect("new file should move into protected source");

    assert_eq!(result.moved_paths, vec![(kick.clone(), moved_kick.clone())]);
    assert!(!kick.exists());
    assert_eq!(fs::read(&moved_kick).expect("read moved kick"), b"kick");
    assert!(
        snare.is_file(),
        "conflicting source should remain unresolved"
    );
    assert_eq!(
        fs::read(&protected_snare).expect("read protected destination"),
        b"protected snare"
    );
    assert_eq!(browser.pending_file_move_conflict_count(), 1);

    let overwrite_error =
        submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Overwrite)
            .expect_err("protected destination overwrite should be blocked");
    assert_eq!(
        overwrite_error,
        "This source is protected. Copy to Primary and continue?"
    );
    assert_eq!(browser.pending_file_move_conflict_count(), 1);
    assert_eq!(fs::read(&snare).expect("read source snare"), b"snare");
    assert_eq!(
        fs::read(&protected_snare).expect("read protected destination"),
        b"protected snare"
    );
    let _ = fs::remove_dir_all(source_root);
    let _ = fs::remove_dir_all(protected_root);
    let _ = fs::remove_dir_all(protected_db_root);
}

#[test]
fn file_drag_drop_defers_destination_name_conflicts() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let existing_kick = loops.join("kick.wav");
    for file in [&kick, &snare, &existing_kick] {
        fs::write(file, file.display().to_string()).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&kick));
    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    let result = submit_folder_drop(&mut browser, &path_id(&loops))
        .expect("non-conflicting files should still move");

    let moved_snare = loops.join("snare.wav");
    assert_eq!(
        result.moved_paths,
        vec![(snare.clone(), moved_snare.clone())]
    );
    assert!(kick.is_file());
    assert!(!snare.exists());
    assert!(moved_snare.is_file());
    assert_eq!(browser.pending_file_move_conflict_count(), 1);
    let conflict = browser
        .pending_file_move_conflict_view()
        .expect("conflict dialog state");
    assert_eq!(conflict.file_name, "kick.wav");
    assert_eq!(conflict.source_path, kick);
    assert_eq!(conflict.destination_path, existing_kick);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_move_conflict_rename_uses_available_copy_name() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-rename");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let existing = loops.join("kick.wav");
    let first_copy = loops.join("kick_copy001.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&existing, b"existing").expect("write existing");
    fs::write(&first_copy, b"copy").expect("write copy");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&source));

    browser.begin_file_drag(path_id(&source), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflict");
    let result = submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Rename)
        .expect("rename conflict should move source");

    let renamed = loops.join("kick_copy002.wav");
    assert_eq!(result.moved_paths, vec![(source.clone(), renamed.clone())]);
    assert!(!source.exists());
    assert_eq!(fs::read(&existing).expect("read existing"), b"existing");
    assert_eq!(fs::read(&renamed).expect("read renamed"), b"source");
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(
        browser.selection.selected_folder_ids,
        HashSet::from([path_id(&drums)])
    );
    assert!(browser.selected_file_paths().is_empty());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_move_conflict_overwrite_replaces_destination() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-overwrite");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let destination = loops.join("kick.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&destination, b"destination").expect("write destination");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&source));

    browser.begin_file_drag(path_id(&source), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflict");
    let result = submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Overwrite)
        .expect("overwrite conflict should move source");

    assert_eq!(
        result.moved_paths,
        vec![(source.clone(), destination.clone())]
    );
    assert!(!source.exists());
    assert_eq!(fs::read(&destination).expect("read destination"), b"source");
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(
        browser.selection.selected_folder_ids,
        HashSet::from([path_id(&drums)])
    );
    assert!(browser.selected_file_paths().is_empty());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_move_conflict_skip_leaves_source_and_destination() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-skip");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let source = drums.join("kick.wav");
    let destination = loops.join("kick.wav");
    fs::write(&source, b"source").expect("write source");
    fs::write(&destination, b"destination").expect("write destination");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&source));

    browser.begin_file_drag(path_id(&source), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflict");
    let result = submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Skip)
        .expect("skip conflict should succeed");

    assert!(result.moved_paths.is_empty());
    assert_eq!(fs::read(&source).expect("read source"), b"source");
    assert_eq!(
        fs::read(&destination).expect("read destination"),
        b"destination"
    );
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_move_conflict_without_apply_all_leaves_next_conflict_pending() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-per-file");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&kick, b"source kick").expect("write kick");
    fs::write(&snare, b"source snare").expect("write snare");
    fs::write(loops.join("kick.wav"), b"existing kick").expect("write existing kick");
    fs::write(loops.join("snare.wav"), b"existing snare").expect("write existing snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    select_two_files_for_move(&mut browser, &drums, &kick, &snare);

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflicts");
    submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Skip)
        .expect("skip first conflict");

    let conflict = browser
        .pending_file_move_conflict_view()
        .expect("unchecked resolution should leave next prompt");
    assert_eq!(conflict.current_number, 2);
    assert_eq!(conflict.total_count, 2);
    assert_eq!(browser.pending_file_move_conflict_count(), 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_move_conflict_apply_all_overwrite_resolves_remaining_conflicts() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-overwrite-all");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let existing_kick = loops.join("kick.wav");
    let existing_snare = loops.join("snare.wav");
    fs::write(&kick, b"source kick").expect("write kick");
    fs::write(&snare, b"source snare").expect("write snare");
    fs::write(&existing_kick, b"existing kick").expect("write existing kick");
    fs::write(&existing_snare, b"existing snare").expect("write existing snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    select_two_files_for_move(&mut browser, &drums, &kick, &snare);

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflicts");
    let result = submit_file_move_conflict(
        &mut browser,
        FileMoveConflictResolutionRequest::apply_to_remaining(
            FileMoveConflictResolution::Overwrite,
        ),
    )
    .expect("overwrite all conflicts");

    assert_eq!(result.moved_paths.len(), 2);
    assert_eq!(fs::read(&existing_kick).expect("read kick"), b"source kick");
    assert_eq!(
        fs::read(&existing_snare).expect("read snare"),
        b"source snare"
    );
    assert!(!kick.exists());
    assert!(!snare.exists());
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_move_conflict_apply_all_skip_resets_for_later_batch() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-skip-all-reset");
    let drums = root.join("drums");
    let loops = root.join("loops");
    let oneshots = root.join("oneshots");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::create_dir_all(&oneshots).expect("create oneshots folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&kick, b"source kick").expect("write kick");
    fs::write(&snare, b"source snare").expect("write snare");
    for target in [&loops, &oneshots] {
        fs::write(target.join("kick.wav"), b"existing kick").expect("write target kick");
        fs::write(target.join("snare.wav"), b"existing snare").expect("write target snare");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    select_two_files_for_move(&mut browser, &drums, &kick, &snare);

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("first drop should park conflicts");
    submit_file_move_conflict(
        &mut browser,
        FileMoveConflictResolutionRequest::apply_to_remaining(FileMoveConflictResolution::Skip),
    )
    .expect("skip all conflicts");
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    assert_eq!(fs::read(&kick).expect("read kick"), b"source kick");
    assert_eq!(fs::read(&snare).expect("read snare"), b"source snare");

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&oneshots))
        .expect("second drop should park new conflicts");
    let conflict = browser
        .pending_file_move_conflict_view()
        .expect("new batch should still prompt");
    assert_eq!(conflict.current_number, 1);
    assert_eq!(conflict.total_count, 2);
    assert_eq!(browser.pending_file_move_conflict_count(), 2);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_move_conflict_apply_all_policy_resets_after_error() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-apply-all-error");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let existing_kick = loops.join("kick.wav");
    let existing_snare = loops.join("snare.wav");
    fs::write(&kick, b"source kick").expect("write kick");
    fs::write(&snare, b"source snare").expect("write snare");
    fs::write(&existing_kick, b"existing kick").expect("write existing kick");
    fs::write(&existing_snare, b"existing snare").expect("write existing snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    select_two_files_for_move(&mut browser, &drums, &kick, &snare);

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflicts");
    fs::remove_file(&snare).expect("remove second source before resolving");
    let result = submit_file_move_conflict(
        &mut browser,
        FileMoveConflictResolutionRequest::apply_to_remaining(
            FileMoveConflictResolution::Overwrite,
        ),
    );
    assert!(result.is_err());
    assert_eq!(fs::read(&existing_kick).expect("read kick"), b"source kick");
    assert_eq!(
        fs::read(&existing_snare).expect("read snare"),
        b"existing snare"
    );

    submit_file_move_conflict(&mut browser, FileMoveConflictResolution::Skip)
        .expect("retry should use the new per-conflict resolution");

    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_move_conflict_apply_all_rename_uses_unique_name_per_conflict() {
    let root = temp_source_root("wavecrate-gui-file-drag-conflict-rename-all");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&kick, b"source kick").expect("write kick");
    fs::write(&snare, b"source snare").expect("write snare");
    fs::write(loops.join("kick.wav"), b"existing kick").expect("write existing kick");
    fs::write(loops.join("kick_copy001.wav"), b"first kick copy")
        .expect("write existing kick copy");
    fs::write(loops.join("snare.wav"), b"existing snare").expect("write existing snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    select_two_files_for_move(&mut browser, &drums, &kick, &snare);

    browser.begin_file_drag(path_id(&kick), Point::new(4.0, 8.0));
    submit_folder_drop(&mut browser, &path_id(&loops)).expect("drop should park conflicts");
    let result = submit_file_move_conflict(
        &mut browser,
        FileMoveConflictResolutionRequest::apply_to_remaining(FileMoveConflictResolution::Rename),
    )
    .expect("rename all conflicts");

    let renamed_kick = loops.join("kick_copy002.wav");
    let renamed_snare = loops.join("snare_copy001.wav");
    assert_eq!(
        result.moved_paths,
        vec![
            (kick.clone(), renamed_kick.clone()),
            (snare.clone(), renamed_snare.clone())
        ]
    );
    assert_eq!(
        fs::read(&renamed_kick).expect("read renamed kick"),
        b"source kick"
    );
    assert_eq!(
        fs::read(&renamed_snare).expect("read renamed snare"),
        b"source snare"
    );
    assert_eq!(browser.pending_file_move_conflict_count(), 0);
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert_eq!(
        browser.selection.selected_folder_ids,
        HashSet::from([path_id(&drums)])
    );
    assert!(browser.selected_file_paths().is_empty());
    let _ = fs::remove_dir_all(root);
}

fn select_two_files_for_move(
    browser: &mut FolderBrowserState,
    folder: &std::path::Path,
    first: &std::path::Path,
    second: &std::path::Path,
) {
    browser.activate_folder(path_id(folder));
    browser.select_file(path_id(first));
    browser.select_file_with_modifiers(
        path_id(second),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
}

fn submit_cut_paste(
    browser: &mut FolderBrowserState,
    file_ids: &[String],
    target_folder_id: &str,
) -> Result<FolderDropResult, String> {
    match browser.prepare_paste_cut_files_to_folder(file_ids, target_folder_id)? {
        FolderMoveDropInput::Status(result) => Ok(result),
        FolderMoveDropInput::Request(request) => {
            let completion = execute_folder_move_request(request);
            let tags_by_file = HashMap::new();
            completion.result.and_then(|success| {
                browser.apply_folder_move_completion(&completion.request, success, &tags_by_file)
            })
        }
    }
}

fn submit_source_drop(
    browser: &mut FolderBrowserState,
    target_source_id: &str,
) -> Result<FolderDropResult, String> {
    match browser.drop_drag_on_source(target_source_id)? {
        FolderMoveDropInput::Status(result) => Ok(result),
        FolderMoveDropInput::Request(request) => {
            let completion = execute_folder_move_request(request);
            let tags_by_file = HashMap::new();
            completion.result.and_then(|success| {
                browser.apply_folder_move_completion(&completion.request, success, &tags_by_file)
            })
        }
    }
}

fn load_source_for_test(browser: &mut FolderBrowserState, source_id: &str, task_id: u64) {
    let Some(request) = browser.begin_select_source(source_id.to_string(), task_id) else {
        return;
    };
    let result = scan_source_with_progress(request, |_| {}, |_| {});
    assert!(browser.apply_scan_finished(result));
}
