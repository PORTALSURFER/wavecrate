use super::*;

#[test]
fn apply_folder_sample_move_result_reports_cancelled_and_noop_statuses() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    controller
        .drag_drop()
        .apply_folder_sample_move_result(FolderSampleMoveResult {
            source_id: source.id.clone(),
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: true,
        });
    assert_eq!(controller.ui.status.text, "Move cancelled");

    controller
        .drag_drop()
        .apply_folder_sample_move_result(FolderSampleMoveResult {
            source_id: source.id,
            moved: Vec::new(),
            errors: Vec::new(),
            cancelled: false,
        });
    assert_eq!(controller.ui.status.text, "No samples moved");
}

#[test]
fn apply_folder_sample_move_result_reports_first_error_when_nothing_moved() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    controller
        .drag_drop()
        .apply_folder_sample_move_result(FolderSampleMoveResult {
            source_id: source.id,
            moved: Vec::new(),
            errors: vec![String::from("A file already exists at dest/one.wav")],
            cancelled: false,
        });

    assert_eq!(
        controller.ui.status.text,
        "A file already exists at dest/one.wav"
    );
}

#[test]
fn apply_folder_move_result_remaps_folder_state_and_focuses_destination() {
    let temp = tempdir().unwrap();
    let _guard = ConfigBaseGuard::set(temp.path().to_path_buf());
    let root = temp.path().join("source");
    std::fs::create_dir_all(root.join("old/manual")).unwrap();
    std::fs::create_dir_all(root.join("dest/old/manual")).unwrap();
    let renderer = WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old/clip.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    controller.toggle_show_all_folders();

    {
        let model = controller.current_folder_model_mut().unwrap();
        model.selected.insert(PathBuf::from("old"));
        model.expanded.insert(PathBuf::from("old"));
        model.focused = Some(PathBuf::from("old"));
        model.selection_anchor = Some(PathBuf::from("old"));
        model.manual_folders.insert(PathBuf::from("old/manual"));
    }
    controller.ui.sources.folders.last_focused_path = Some(PathBuf::from("old"));

    controller
        .drag_drop()
        .apply_folder_move_result(FolderMoveResult {
            source_id: source.id,
            old_folder: PathBuf::from("old"),
            new_folder: PathBuf::from("dest/old"),
            folder_moved: true,
            moved: vec![FolderEntryMove {
                old_relative: PathBuf::from("old/clip.wav"),
                new_relative: PathBuf::from("dest/old/clip.wav"),
                file_size: 0,
                modified_ns: 0,
                tag: crate::sample_sources::Rating::NEUTRAL,
                looped: false,
                locked: false,
                last_played_at: None,
            }],
            errors: Vec::new(),
            cancelled: false,
        });

    let model = controller.current_folder_model().unwrap();
    assert!(model.selected.contains(Path::new("dest/old")));
    assert!(model.expanded.contains(Path::new("dest/old")));
    assert_eq!(model.focused.as_deref(), Some(Path::new("dest/old")));
    assert_eq!(
        model.selection_anchor.as_deref(),
        Some(Path::new("dest/old"))
    );
    assert!(model.manual_folders.contains(Path::new("dest/old/manual")));
    assert_eq!(
        controller.ui.sources.folders.last_focused_path.as_deref(),
        Some(Path::new("dest/old"))
    );
    assert_eq!(controller.ui.status.text, "Moved folder to dest/old");
}
