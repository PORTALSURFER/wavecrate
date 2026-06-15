use super::*;
use radiant::prelude as ui;

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
fn file_scroll_tracking_is_not_overridden_by_unchanged_selection_follow() {
    let root = temp_source_root("wavecrate-gui-file-scroll-stable-after-follow");
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
    browser.select_file(path_id(&files[8]));

    assert_eq!(browser.follow_selected_file_view(6, 1, 1).viewport_start, 5);
    browser.set_file_view_start_from_scroll_offset(20.0 * 22.0, 22.0);

    assert_eq!(
        browser.follow_selected_file_view(6, 1, 1).viewport_start,
        18
    );
    assert_eq!(browser.file_view_start(), 18);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_scroll_tracking_uses_runtime_viewport_rows_after_scrollbar_update() {
    let root = temp_source_root("wavecrate-gui-file-scroll-runtime-viewport");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let files = (0..80)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&files[4]));

    let initial = browser.follow_selected_file_view_matching_tags(128, 4, 2, &Default::default());
    assert_eq!(initial.viewport_len(), 80);

    browser.apply_file_view_window_change(ui::VirtualListWindowChange {
        offset_y: 40.0 * 22.0,
        row_height: 22.0,
        window: ui::VirtualListWindow {
            total_items: 80,
            viewport_start: 40,
            viewport_end: 58,
            window_start: 36,
            window_end: 62,
        },
    });

    let scrolled = browser.follow_selected_file_view_matching_tags(128, 4, 2, &Default::default());

    assert_eq!(scrolled.viewport_start, 40);
    assert_eq!(scrolled.viewport_len(), 18);
    assert_eq!(scrolled.window_start, 36);
    assert_eq!(scrolled.window_end, 62);
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

    let window = browser.sync_tree_view_to_selection(6, 1, 1);

    assert_eq!(window.viewport_start, 10);
    assert_eq!(browser.tree_view_start(), 10);
    let _ = fs::remove_dir_all(root);
}
#[test]
fn folder_tree_scroll_tracking_is_not_overridden_by_unchanged_selection_follow() {
    let root = temp_source_root("wavecrate-gui-folder-tree-scroll-stable-after-follow");
    for index in 0..24 {
        fs::create_dir_all(root.join(format!("folder_{index:02}"))).expect("create folder");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&root.join("folder_12")));

    assert_eq!(
        browser.sync_tree_view_to_selection(6, 1, 1).viewport_start,
        10
    );
    browser.set_tree_view_start_from_scroll_offset(
        20.0 * super::super::TREE_ROW_HEIGHT,
        super::super::TREE_ROW_HEIGHT,
    );

    assert_eq!(
        browser.sync_tree_view_to_selection(6, 1, 1).viewport_start,
        19
    );
    assert_eq!(browser.tree_view_start(), 19);
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
