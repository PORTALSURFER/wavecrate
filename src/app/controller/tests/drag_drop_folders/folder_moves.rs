use super::*;

#[test]
fn folder_drop_to_folder_moves_tree() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let src_folder = source.root.join("one");
    let dest_folder = source.root.join("dest");
    std::fs::create_dir_all(&src_folder).unwrap();
    std::fs::create_dir_all(&dest_folder).unwrap();
    write_test_wav(&src_folder.join("clip.wav"), &[0.1, 0.2]);

    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();

    controller.ui.drag.payload = Some(DragPayload::Folder {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("one"),
    });
    controller.ui.drag.set_target(
        DragSource::Folders,
        DragTarget::FolderPanel {
            folder: Some(PathBuf::from("dest")),
        },
    );
    controller.finish_active_drag();

    assert!(!src_folder.exists());
    assert!(source.root.join("dest/one/clip.wav").is_file());
    assert_eq!(
        controller.wav_entry(0).unwrap().relative_path,
        PathBuf::from("dest/one/clip.wav")
    );
}

#[test]
fn folder_drop_to_folder_rejects_selected_source_mismatch() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root_a = temp.path().join("source_a");
    let root_b = temp.path().join("source_b");
    std::fs::create_dir_all(root_a.join("old")).unwrap();
    std::fs::create_dir_all(&root_b).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source_a = SampleSource::new(root_a);
    let source_b = SampleSource::new(root_b);
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.selection_state.ctx.selected_source = Some(source_b.id.clone());

    controller.drag_drop().handle_folder_drop_to_folder(
        source_a.id.clone(),
        PathBuf::from("old"),
        Path::new("dest"),
    );

    assert_eq!(
        controller.ui.status.text,
        "Switch to the folder's source before moving it"
    );
    assert!(controller.ui.progress.task.is_none());
}

#[test]
fn folder_drop_to_folder_rejects_root_self_and_descendant_targets() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    std::fs::create_dir_all(root.join("old/sub")).unwrap();
    std::fs::create_dir_all(root.join("dest")).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root);
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());

    controller.drag_drop().handle_folder_drop_to_folder(
        source.id.clone(),
        PathBuf::new(),
        Path::new("dest"),
    );
    assert_eq!(controller.ui.status.text, "Root folder cannot be moved");

    controller.drag_drop().handle_folder_drop_to_folder(
        source.id.clone(),
        PathBuf::from("old"),
        Path::new("old"),
    );
    assert_eq!(controller.ui.status.text, "Folder is already there");

    controller.drag_drop().handle_folder_drop_to_folder(
        source.id.clone(),
        PathBuf::from("old"),
        Path::new("old/sub"),
    );
    assert_eq!(
        controller.ui.status.text,
        "Cannot move a folder into itself"
    );
}
