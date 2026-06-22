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
fn file_drag_drop_moves_explicit_selection_when_focus_has_advanced() {
    let root = temp_source_root("wavecrate-gui-file-drag-explicit-focus-advanced");
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
        .toggle_focused_sample_selection_and_advance(&Default::default())
        .expect("mark first file");
    browser.navigate_vertical(1, false);
    browser
        .toggle_focused_sample_selection_and_advance(&Default::default())
        .expect("mark third file");
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
