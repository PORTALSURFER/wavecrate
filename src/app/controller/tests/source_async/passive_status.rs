use super::*;
#[test]
fn cached_source_hydration_does_not_launch_passive_background_scan() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    let hydrated_entries = vec![sample_entry("cached.wav", Rating::NEUTRAL)];
    std::fs::write(sources[1].root.join("cached.wav"), b"fixture").unwrap();

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index(1);
        let request_id = controller
            .runtime
            .source_lane
            .hydration
            .pending_active
            .as_ref()
            .expect("pending source hydration")
            .request_id;
        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[1],
                request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                hydrated_entries.clone(),
                true,
            ),
        ));
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let db =
        crate::sample_sources::SourceDatabase::open_for_test_fixture_source_write(&sources[1].root)
            .unwrap();
    assert_eq!(db.count_files().unwrap(), 0);
}

#[test]
fn analysis_only_busy_metadata_failure_does_not_overwrite_status() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a"]);
    let source = sources[0].clone();
    controller.ui.status.text = String::from("Auto Rename: renamed 1, skipped 0, failed 0");
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(PendingMetadataMutation {
            request_id: 7,
            source_id: source.id.clone(),
            paths: [PathBuf::from("alpha.wav")].into_iter().collect(),
            blocks_file_mutation: false,
            rollback: Vec::new(),
            refresh_browser_projection: false,
        });

    controller.apply_background_job_message_for_tests(JobMessage::MetadataMutationFinished(
        MetadataMutationResult {
            request_id: 7,
            source_id: source.id,
            paths: [PathBuf::from("alpha.wav")].into_iter().collect(),
            elapsed: std::time::Duration::from_millis(5),
            result: Err(String::from(
                "Failed to start analysis metadata transaction: database is locked",
            )),
        },
    ));

    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 1, skipped 0, failed 0"
    );
}
