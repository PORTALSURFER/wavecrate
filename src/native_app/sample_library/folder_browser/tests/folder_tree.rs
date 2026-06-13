use super::*;
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

    assert_eq!(browser.selection.selected_folder, path_id(&root));
    assert!(browser.navigate_selected_folder(1));
    assert_eq!(browser.selection.selected_folder, path_id(&drums));
    assert!(!browser.is_expanded(&path_id(&drums)));
    assert!(browser.expand_selected_folder());
    assert!(browser.is_expanded(&path_id(&drums)));
    assert!(browser.collapse_selected_folder());
    assert!(!browser.is_expanded(&path_id(&drums)));
    assert!(browser.expand_selected_folder());
    assert!(browser.is_expanded(&path_id(&drums)));
    assert!(browser.navigate_selected_folder(1));
    assert_eq!(browser.selection.selected_folder, path_id(&kicks));
    assert!(browser.navigate_selected_folder(1));
    assert_eq!(browser.selection.selected_folder, path_id(&snares));
    assert!(!browser.navigate_selected_folder(1));
    assert_eq!(browser.selection.selected_folder, path_id(&snares));

    let _ = fs::remove_dir_all(root);
}
#[test]
fn folder_audio_projection_cache_is_prewarmed_for_loaded_source_tree() {
    let root = temp_source_root("wavecrate-gui-folder-audio-projection-cache");
    let kicks = root.join("kicks");
    let snares = root.join("snares");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&snares).expect("create snares folder");
    fs::write(kicks.join("kick.wav"), []).expect("write kick");
    fs::write(snares.join("snare.wav"), []).expect("write snare");

    let browser = FolderBrowserState::from_root(root.clone());

    assert!(
        browser.selected_audio_projection_cache_len_for_tests() >= 3,
        "source load should prewarm root and child folder audio projections"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn visible_folders_mark_branches_without_audio_as_empty() {
    let root = temp_source_root("wavecrate-gui-folder-empty-state");
    let empty = root.join("empty");
    let parent = root.join("parent");
    let child = parent.join("child");
    let direct = root.join("direct");
    fs::create_dir_all(&empty).expect("create empty folder");
    fs::create_dir_all(&child).expect("create nested folder");
    fs::create_dir_all(&direct).expect("create direct folder");
    fs::write(child.join("nested.wav"), []).expect("write nested audio");
    fs::write(direct.join("direct.wav"), []).expect("write direct audio");

    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&parent));
    let visible = browser.visible_folders();

    let empty_row = visible_folder_by_id(&visible, &empty);
    assert!(empty_row.empty);
    let parent_row = visible_folder_by_id(&visible, &parent);
    assert!(
        !parent_row.empty,
        "audio descendants make a branch non-empty"
    );
    let child_row = visible_folder_by_id(&visible, &child);
    assert!(!child_row.empty);
    let direct_row = visible_folder_by_id(&visible, &direct);
    assert!(!direct_row.empty);

    let _ = fs::remove_dir_all(root);
}

fn visible_folder_by_id<'a>(
    visible: &'a [crate::native_app::sample_library::folder_browser::model::VisibleFolder],
    path: &std::path::Path,
) -> &'a crate::native_app::sample_library::folder_browser::model::VisibleFolder {
    let id = path_id(path);
    visible
        .iter()
        .find(|folder| folder.id == id)
        .expect("visible folder should exist")
}

#[test]
fn source_root_folder_is_static_dot_selector() {
    let root = temp_source_root("wavecrate-gui-root-dot-selector");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let root_id = path_id(&root);

    let visible = browser.visible_folders();
    let root_row = visible
        .iter()
        .find(|folder| folder.id == root_id)
        .expect("root row should be visible");
    assert_eq!(root_row.name, ".");
    assert!(root_row.is_source_root);
    assert!(root_row.expanded);
    assert!(
        visible.iter().any(|folder| folder.id == path_id(&drums)),
        "root children should stay visible without expanding the root row"
    );

    assert!(!browser.collapse_selected_folder());
    browser.activate_folder(root_id.clone());
    assert_eq!(browser.selection.selected_folder, root_id);
    assert!(
        browser
            .visible_folders()
            .iter()
            .any(|folder| folder.id == path_id(&drums)),
        "activating root should select it without collapsing its children"
    );
    assert!(!browser.expand_selected_folder());

    let _ = fs::remove_dir_all(root);
}
#[test]
fn folder_expander_toggles_without_selecting_folder() {
    let root = temp_source_root("wavecrate-gui-folder-expander-toggle");
    let alpha = root.join("alpha");
    let nested = alpha.join("nested");
    let beta = root.join("beta");
    fs::create_dir_all(&nested).expect("create nested folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let alpha_id = path_id(&alpha);
    let beta_id = path_id(&beta);

    browser.activate_folder(beta_id.clone());
    assert_eq!(browser.selection.selected_folder, beta_id);
    assert!(!browser.is_expanded(&alpha_id));

    browser.apply_message(FolderBrowserMessage::ToggleFolderExpansion(
        alpha_id.clone(),
    ));

    assert_eq!(browser.selection.selected_folder, path_id(&beta));
    assert!(browser.is_expanded(&alpha_id));

    browser.apply_message(FolderBrowserMessage::ToggleFolderExpansion(
        alpha_id.clone(),
    ));

    assert_eq!(browser.selection.selected_folder, path_id(&beta));
    assert!(!browser.is_expanded(&alpha_id));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn source_root_expander_toggle_is_ignored() {
    let root = temp_source_root("wavecrate-gui-root-expander-toggle-ignored");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let root_id = path_id(&root);

    browser.apply_message(FolderBrowserMessage::ToggleFolderExpansion(root_id.clone()));

    assert!(browser.is_expanded(&root_id));
    assert_eq!(browser.selection.selected_folder, root_id);
    assert!(
        browser
            .visible_folders()
            .iter()
            .any(|folder| folder.id == path_id(&drums)),
        "source root children should remain visible"
    );
    let _ = fs::remove_dir_all(root);
}
