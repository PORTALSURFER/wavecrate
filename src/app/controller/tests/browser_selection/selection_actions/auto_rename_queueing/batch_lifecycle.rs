use super::*;

#[test]
fn active_auto_rename_batch_tracks_progress_remaps_and_clears_on_finish() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.show_status_progress(
        crate::app::state::ProgressTaskKind::FileOps,
        "Preparing auto rename",
        2,
        true,
    );
    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(BrowserRenameIntentKey::new(
            source.id.clone(),
            vec![
                (PathBuf::from("alpha.wav"), PathBuf::from("alpha.wav")),
                (PathBuf::from("beta.wav"), PathBuf::from("beta.wav")),
            ],
        ));
    controller
        .runtime
        .source_lane
        .mutations
        .begin_auto_rename_batch(
            source.id.clone(),
            vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")],
        );
    let (_tx, rx) = std::sync::mpsc::channel();
    controller.runtime.jobs.start_file_ops(
        rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    assert_auto_rename_row(
        &controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .expect("active batch")
            .rows[0],
        "alpha.wav",
        "alpha.wav",
        AutoRenameBatchRowState::Queued,
    );

    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Active {
            old_relative: PathBuf::from("alpha.wav"),
        });
    let snapshot = controller
        .runtime
        .source_lane
        .mutations
        .active_auto_rename_batch_snapshot()
        .expect("active batch after start");
    assert_eq!(snapshot.current_path, Some(PathBuf::from("alpha.wav")));
    assert_auto_rename_row(
        &snapshot.rows[0],
        "alpha.wav",
        "alpha.wav",
        AutoRenameBatchRowState::Active,
    );

    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Completed {
            old_relative: PathBuf::from("alpha.wav"),
            new_relative: PathBuf::from("alpha_renamed.wav"),
        });
    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Active {
            old_relative: PathBuf::from("beta.wav"),
        });
    controller
        .runtime
        .source_lane
        .mutations
        .apply_auto_rename_progress(SampleAutoRenameProgress::Failed {
            old_relative: PathBuf::from("beta.wav"),
            error: String::from("Disk error"),
        });

    let snapshot = controller
        .runtime
        .source_lane
        .mutations
        .active_auto_rename_batch_snapshot()
        .expect("active batch after item progress");
    assert_eq!(snapshot.current_path, None);
    assert_eq!(
        snapshot.remaps,
        vec![(
            PathBuf::from("alpha.wav"),
            PathBuf::from("alpha_renamed.wav")
        )]
    );
    assert_auto_rename_row(
        &snapshot.rows[0],
        "alpha.wav",
        "alpha_renamed.wav",
        AutoRenameBatchRowState::Completed,
    );
    assert_auto_rename_row(
        &snapshot.rows[1],
        "beta.wav",
        "beta.wav",
        AutoRenameBatchRowState::Failed,
    );

    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id: source.id,
                requested_paths: vec![PathBuf::from("alpha.wav"), PathBuf::from("beta.wav")],
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: vec![(PathBuf::from("beta.wav"), String::from("Disk error"))],
            },
        ),
    );

    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_none()
    );
}

#[test]
fn active_auto_rename_batch_clears_when_selected_source_changes() {
    let (mut controller, first) = dummy_controller();
    let second_temp = tempfile::tempdir().unwrap();
    let second = crate::sample_sources::SampleSource::new(second_temp.path().to_path_buf());
    controller.library.sources.push(first.clone());
    controller.library.sources.push(second.clone());
    controller.select_source_by_index(0);
    controller
        .runtime
        .source_lane
        .mutations
        .begin_auto_rename_batch(first.id.clone(), vec![PathBuf::from("alpha.wav")]);

    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_some()
    );

    controller.select_source_by_index(1);

    assert!(
        controller
            .runtime
            .source_lane
            .mutations
            .active_auto_rename_batch_snapshot()
            .is_none()
    );
}
