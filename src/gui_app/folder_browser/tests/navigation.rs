use super::*;
use crate::gui_app::folder_browser::collections::{
    COLLAPSED_COLLECTIONS_PANEL_HEIGHT, MIN_COLLECTIONS_PANEL_HEIGHT,
};

#[test]
fn visible_folder_depths_are_stable_for_siblings() {
    let root = temp_source_root("wavecrate-gui-folder-depths");
    for child in ["alpha", "beta", "gamma"] {
        fs::create_dir_all(root.join("parent").join(child)).expect("create nested folder");
    }
    let browser = FolderBrowserState::from_root(root.clone());
    let mut browser = browser;
    browser.activate_folder(path_id(&root.join("parent")));

    let sibling_depths = browser
        .visible_folders()
        .into_iter()
        .filter(|folder| ["alpha", "beta", "gamma"].contains(&folder.name.as_str()))
        .map(|folder| folder.depth)
        .collect::<Vec<_>>();

    assert_eq!(sibling_depths, vec![2, 2, 2]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_keyboard_navigation_moves_visible_selection_and_expands_collapses() {
    let root = temp_source_root("wavecrate-gui-folder-keyboard");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    let snares = drums.join("snares");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&snares).expect("create snares folder");
    let mut browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(browser.selected_folder, path_id(&root));
    assert!(browser.navigate_selected_folder(1));
    assert_eq!(browser.selected_folder, path_id(&drums));
    assert!(!browser.is_expanded(&path_id(&drums)));
    assert!(browser.expand_selected_folder());
    assert!(browser.is_expanded(&path_id(&drums)));
    assert!(browser.collapse_selected_folder());
    assert!(!browser.is_expanded(&path_id(&drums)));
    assert!(browser.expand_selected_folder());
    assert!(browser.is_expanded(&path_id(&drums)));
    assert!(browser.navigate_selected_folder(1));
    assert_eq!(browser.selected_folder, path_id(&kicks));
    assert!(browser.navigate_selected_folder(1));
    assert_eq!(browser.selected_folder, path_id(&snares));
    assert!(!browser.navigate_selected_folder(1));
    assert_eq!(browser.selected_folder, path_id(&snares));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn collections_panel_splitter_resizes_and_clamps_height() {
    let root = temp_source_root("wavecrate-gui-collections-panel-resize");
    let mut browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(
        browser.collections_panel_height,
        super::super::DEFAULT_COLLECTIONS_PANEL_HEIGHT
    );

    browser.resize_collections_panel(DragHandleMessage::Started {
        position: Point::new(0.0, 200.0),
    });
    browser.resize_collections_panel(DragHandleMessage::Moved {
        position: Point::new(0.0, 120.0),
    });
    assert_eq!(
        browser.collections_panel_height,
        browser.max_collections_panel_height()
    );

    browser.resize_collections_panel(DragHandleMessage::Moved {
        position: Point::new(0.0, 1_000.0),
    });
    assert_eq!(
        browser.collections_panel_height,
        MIN_COLLECTIONS_PANEL_HEIGHT
    );

    browser.resize_collections_panel(DragHandleMessage::Ended {
        position: Point::new(0.0, -1_000.0),
    });
    assert_eq!(
        browser.collections_panel_height,
        browser.max_collections_panel_height()
    );
    assert!(browser.collection_panel_resize.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn collections_panel_splitter_double_click_collapses_height() {
    let root = temp_source_root("wavecrate-gui-collections-panel-collapse");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.resize_collections_panel(DragHandleMessage::Started {
        position: Point::new(0.0, 200.0),
    });

    browser.resize_collections_panel(DragHandleMessage::DoubleActivate {
        position: Point::new(0.0, 200.0),
    });

    assert_eq!(
        browser.collections_panel_height,
        COLLAPSED_COLLECTIONS_PANEL_HEIGHT
    );
    assert!(browser.collection_panel_resize.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn focus_file_across_sources_reselects_loaded_file_parent_folder() {
    let root = temp_source_root("wavecrate-gui-focus-loaded");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    let loop_file = loops.join("loop.wav");
    fs::write(&kick, []).expect("write kick");
    fs::write(&loop_file, []).expect("write loop");
    let mut browser = FolderBrowserState::from_root(root.clone());

    browser.activate_folder(path_id(&loops));
    browser.select_file(path_id(&loop_file));
    assert_eq!(browser.selected_folder, path_id(&loops));

    assert!(browser.focus_file_across_sources(&kick));

    assert_eq!(browser.selected_folder, path_id(&kicks));
    assert_eq!(browser.selected_file_id(), Some(path_id(&kick).as_str()));
    assert!(browser.is_expanded(&path_id(&root.join("drums"))));
    assert!(browser.is_expanded(&path_id(&kicks)));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn focus_file_across_sources_loads_configured_source_before_selecting_file() {
    let first = temp_source_root("wavecrate-gui-focus-source-first");
    let second = temp_source_root("wavecrate-gui-focus-source-second");
    fs::write(first.join("first.wav"), []).expect("write first sample");
    let nested = second.join("drums");
    fs::create_dir_all(&nested).expect("create nested folder");
    let target = nested.join("target.wav");
    fs::write(&target, []).expect("write target sample");
    let sources = vec![
        wavecrate::sample_sources::SampleSource::new(first.clone()),
        wavecrate::sample_sources::SampleSource::new(second.clone()),
    ];
    let mut browser = FolderBrowserState::from_sample_sources(&sources);

    assert!(
        browser
            .sources
            .iter()
            .find(|source| source.root == second)
            .and_then(|source| source.root_folder.as_ref())
            .is_none()
    );

    assert!(browser.focus_file_across_sources(&target));

    assert_eq!(browser.selected_folder, path_id(&nested));
    assert_eq!(browser.selected_file_id(), Some(path_id(&target).as_str()));
    assert!(
        browser
            .sources
            .iter()
            .find(|source| source.root == second)
            .and_then(|source| source.root_folder.as_ref())
            .is_some()
    );
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn file_keyboard_navigation_moves_audio_selection_without_leaving_folder() {
    let root = temp_source_root("wavecrate-gui-file-keyboard");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&hat, [0_u8; 8]).expect("write hat");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(&snare, [0_u8; 8]).expect("write snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&hat));

    assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&kick)));
    browser.select_file(path_id(&kick));
    assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&snare)));
    browser.select_file(path_id(&snare));
    assert_eq!(browser.navigate_vertical(1, false), None);
    assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));
    assert_eq!(browser.navigate_vertical(-1, false), Some(path_id(&kick)));
    assert_eq!(browser.selected_folder, path_id(&drums));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_keyboard_navigation_can_extend_audio_selection() {
    let root = temp_source_root("wavecrate-gui-file-keyboard-extend");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&hat, [0_u8; 8]).expect("write hat");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(&snare, [0_u8; 8]).expect("write snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&hat));

    assert_eq!(browser.navigate_vertical(1, true), Some(path_id(&kick)));
    assert_eq!(browser.navigate_vertical(1, true), Some(path_id(&snare)));

    assert_eq!(
        browser.selected_file_paths(),
        vec![hat.clone(), kick.clone(), snare.clone()]
    );
    assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_mouse_selection_toggles_and_extends_audio_selection() {
    let root = temp_source_root("wavecrate-gui-file-mouse-multi-select");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
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

    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![hat.clone(), snare.clone()]
    );

    browser.select_file_with_modifiers(
        path_id(&tom),
        PointerModifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![snare.clone(), tom.clone()]
    );

    browser.select_file_with_modifiers(
        path_id(&kick),
        PointerModifiers {
            command: true,
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![kick.clone(), snare.clone(), tom.clone()]
    );

    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![kick.clone(), tom.clone()]
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_keyboard_navigation_follow_window_moves_only_near_edges() {
    let root = temp_source_root("wavecrate-gui-file-follow-window");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let files = (0..20)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&files[4]));

    let window = browser.follow_selected_file_view(6, 1, 1);
    assert_eq!(window.viewport_start, 1);
    assert_eq!(browser.file_view_start(), 1);

    assert_eq!(
        browser.navigate_vertical(1, false),
        Some(path_id(&files[5]))
    );
    let window = browser.follow_selected_file_view(6, 1, 1);
    assert_eq!(window.viewport_start, 2);
    assert_eq!(browser.file_view_start(), 2);

    assert_eq!(
        browser.navigate_vertical(1, false),
        Some(path_id(&files[6]))
    );
    let window = browser.follow_selected_file_view(6, 1, 1);
    assert_eq!(window.viewport_start, 3);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_scroll_tracking_allows_runtime_clamped_bottom_offsets() {
    let root = temp_source_root("wavecrate-gui-file-scroll-bottom");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let files = (0..24)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.set_file_view_start_from_scroll_offset(23.0 * 22.0, 22.0);

    assert_eq!(browser.file_view_start(), 23);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_tree_follow_window_tracks_selected_folder() {
    let root = temp_source_root("wavecrate-gui-folder-tree-follow-window");
    for index in 0..20 {
        fs::create_dir_all(root.join(format!("folder_{index:02}"))).expect("create folder");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("folder_12")));
    let visible = browser.visible_folders();
    let selected = visible.iter().position(|folder| folder.selected);

    let window = browser.follow_selected_tree_view(visible.len(), selected, 6, 1, 1);

    assert_eq!(window.viewport_start, 10);
    assert_eq!(browser.tree_view_start(), 10);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_tree_scroll_tracking_allows_runtime_offsets() {
    let root = temp_source_root("wavecrate-gui-folder-tree-scroll");
    for index in 0..24 {
        fs::create_dir_all(root.join(format!("folder_{index:02}"))).expect("create folder");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());

    browser.set_tree_view_start_from_scroll_offset(
        23.0 * super::super::TREE_ROW_HEIGHT,
        super::super::TREE_ROW_HEIGHT,
    );

    assert_eq!(browser.tree_view_start(), 23);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn select_all_audio_files_selects_current_folder_samples() {
    let root = temp_source_root("wavecrate-gui-file-select-all");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let note = drums.join("note.txt");
    fs::write(&hat, [0_u8; 8]).expect("write hat");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(&note, [0_u8; 8]).expect("write note");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    assert_eq!(browser.select_all_audio_files(), 2);

    assert_eq!(
        browser.selected_file_paths(),
        vec![hat.clone(), kick.clone()]
    );
    assert!(!browser.is_file_selected(&path_id(&note)));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn first_audio_file_path_finds_first_audio_in_selected_source_tree() {
    let root = temp_source_root("wavecrate-gui-first-startup-audio");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    fs::write(root.join("readme.txt"), []).expect("write text file");
    let first = alpha.join("a_first.wav");
    let second = beta.join("b_second.wav");
    fs::write(&first, []).expect("write first sample");
    fs::write(&second, []).expect("write second sample");

    let browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(browser.first_audio_file_path(), Some(first));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn activating_collection_filters_audio_files_across_selected_source() {
    let root = temp_source_root("wavecrate-gui-collection-filter");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    let beta_other = beta.join("beta_other.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    fs::write(&beta_other, []).expect("write other sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&alpha_keep, collection);
    browser.set_file_collection_state(&beta_keep, collection);

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav", "beta_keep.wav"]
    );
    browser.select_file(path_id(&beta_keep));
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&beta_keep).as_str())
    );

    browser.activate_folder(path_id(&alpha));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
/// Activating a collection transfers active selection out of the folder tree.
fn activating_collection_clears_folder_selection_and_keeps_collection_as_active_source() {
    let root = temp_source_root("wavecrate-gui-collection-clears-folder");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    let beta_other = beta.join("beta_other.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    fs::write(&beta_other, []).expect("write other sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&beta));
    assert_eq!(browser.selected_folder_path(), Some(beta.clone()));

    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&alpha_keep, collection);
    browser.set_file_collection_state(&beta_keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(browser.selected_folder_path(), None);
    assert!(
        browser
            .visible_folders()
            .iter()
            .all(|folder| !folder.selected)
    );
    assert_eq!(
        browser
            .visible_collections()
            .into_iter()
            .find(|view| view.collection == collection)
            .map(|view| view.selected),
        Some(true)
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha_keep.wav", "beta_keep.wav"]
    );

    browser.activate_folder(path_id(&alpha));

    assert_eq!(browser.selected_folder_path(), Some(alpha.clone()));
    assert!(
        browser
            .visible_collections()
            .into_iter()
            .all(|view| !view.selected)
    );
    assert_eq!(
        browser
            .visible_folders()
            .into_iter()
            .find(|folder| folder.id == path_id(&alpha))
            .map(|folder| folder.selected),
        Some(true)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
/// Keyboard navigation in collection mode enters the filtered sample list.
fn collection_navigation_enters_filtered_files_without_reselecting_folder() {
    let root = temp_source_root("wavecrate-gui-collection-keyboard");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let alpha_keep = alpha.join("alpha_keep.wav");
    let beta_keep = beta.join("beta_keep.wav");
    fs::write(&alpha_keep, []).expect("write alpha sample");
    fs::write(&beta_keep, []).expect("write beta sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&beta));
    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&alpha_keep, collection);
    browser.set_file_collection_state(&beta_keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        browser.navigate_vertical(1, false),
        Some(path_id(&alpha_keep))
    );
    assert_eq!(
        browser.selected_file_id(),
        Some(path_id(&alpha_keep).as_str())
    );
    assert_eq!(browser.selected_folder_path(), None);
    assert!(!browser.expand_selected_folder());
    assert!(!browser.collapse_selected_folder());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn repeated_collection_activation_does_not_start_rename() {
    let root = temp_source_root("wavecrate-gui-collection-slow-click");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert!(browser.collection_rename_view(collection).is_none());

    browser.apply_message(FolderBrowserMessage::RenameCollection(collection));

    assert!(browser.collection_rename_view(collection).is_some());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cancel_rename_exits_collection_rename() {
    let root = temp_source_root("wavecrate-gui-collection-cancel-rename");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");
    browser.apply_message(FolderBrowserMessage::RenameCollection(collection));
    assert!(browser.collection_rename_view(collection).is_some());

    browser.apply_message(FolderBrowserMessage::CancelRename);

    assert!(browser.collection_rename_view(collection).is_none());
    assert!(!browser.rename_active());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sample_file_sort_toggles_by_column_and_navigation_uses_sorted_order() {
    let root = temp_source_root("wavecrate-gui-file-sort");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let small = drums.join("small.wav");
    let large = drums.join("large.wav");
    fs::write(&small, [0_u8; 8]).expect("write small");
    fs::write(&large, [0_u8; 128]).expect("write large");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from("size")));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["small.wav", "large.wav"]
    );

    browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from("size")));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["large.wav", "small.wav"]
    );
    browser.select_file(path_id(&large));
    assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&small)));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn sample_file_column_resize_clamps_width() {
    let mut browser = FolderBrowserState::load_default();

    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("extension"),
        radiant::widgets::DragHandleMessage::Started {
            position: Point::new(100.0, 0.0),
        },
    ));
    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("extension"),
        radiant::widgets::DragHandleMessage::Moved {
            position: Point::new(-200.0, 0.0),
        },
    ));

    let extension_width = browser
        .visible_file_columns()
        .into_iter()
        .find(|column| column.id == "extension")
        .map(|column| column.width)
        .unwrap();
    assert_eq!(extension_width, MIN_FILE_COLUMN_WIDTH);
}

#[test]
fn sample_file_column_drag_reorders_columns() {
    let mut browser = FolderBrowserState::load_default();

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::Started {
            position: Point::new(284.0, 0.0),
        },
    ));
    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::Moved {
            position: Point::new(560.0, 0.0),
        },
    ));
    let feedback = browser
        .file_column_drag_feedback()
        .expect("active column drag should project visual feedback");
    assert_eq!(feedback.label, "Rating");
    assert_eq!(feedback.pointer, Point::new(560.0, 0.0));
    assert!(feedback.marker_x > 300.0, "{feedback:?}");

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::Ended {
            position: Point::new(560.0, 0.0),
        },
    ));

    assert_eq!(browser.file_column_drag_feedback(), None);
    assert_eq!(
        browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "collection",
            "extension",
            "size",
            "rating",
            "modified"
        ]
    );
}
