use super::*;
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
    assert_eq!(browser.selection.selected_folder, path_id(&loops));

    assert!(browser.focus_file_across_sources(&kick));

    assert_eq!(browser.selection.selected_folder, path_id(&kicks));
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
fn focus_file_across_sources_preserves_recursive_folder_when_file_is_already_visible() {
    let root = temp_source_root("wavecrate-gui-focus-loaded-visible-recursive");
    let kicks = root.join("drums").join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    let loop_file = loops.join("loop.wav");
    fs::write(&kick, []).expect("write kick");
    fs::write(&loop_file, []).expect("write loop");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.toggle_folder_subtree_listing();
    browser.select_file(path_id(&loop_file));

    assert_eq!(browser.selection.selected_folder, path_id(&root));
    assert!(browser.folder_subtree_listing_enabled());

    assert!(browser.focus_file_across_sources(&kick));

    assert_eq!(browser.selection.selected_folder, path_id(&root));
    assert!(browser.folder_subtree_listing_enabled());
    assert_eq!(browser.selected_file_id(), Some(path_id(&kick).as_str()));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "loop.wav"]
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
            .source
            .sources
            .iter()
            .find(|source| source.root == second)
            .and_then(|source| source.root_folder.as_ref())
            .is_none()
    );

    assert!(browser.focus_file_across_sources(&target));

    assert_eq!(browser.selection.selected_folder, path_id(&nested));
    assert_eq!(browser.selected_file_id(), Some(path_id(&target).as_str()));
    assert!(
        browser
            .source
            .sources
            .iter()
            .find(|source| source.root == second)
            .and_then(|source| source.root_folder.as_ref())
            .is_some()
    );
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}
