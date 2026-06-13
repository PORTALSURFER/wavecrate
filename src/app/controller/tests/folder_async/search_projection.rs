use super::*;

#[test]
fn folder_search_queues_async_projection_and_preserves_latest_query() {
    let (mut controller, source) = nested_folder_controller();
    let rows_before = visible_folder_paths(&controller);

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.set_folder_search(String::from("kic"));
        let first_request_id = pending_projection_request_id(&controller);
        assert_eq!(visible_folder_paths(&controller), rows_before);

        controller.set_folder_search(String::from("dru"));
        let second_request_id = pending_projection_request_id(&controller);

        controller.apply_background_job_message_for_tests(JobMessage::FolderProjected(
            projection_result(
                &controller,
                &source,
                first_request_id,
                vec![row("drums/kicks", 2, false, false)],
                Some(0),
            ),
        ));
        assert_eq!(controller.ui.sources.folders.search_query, "dru");
        assert!(
            controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Upper)
                .projecting
        );

        controller.apply_background_job_message_for_tests(JobMessage::FolderProjected(
            projection_result(
                &controller,
                &source,
                second_request_id,
                vec![row("drums", 1, true, true)],
                Some(0),
            ),
        ));
    });

    assert_eq!(controller.ui.sources.folders.search_query, "dru");
    assert_eq!(controller.ui.sources.folders.rows.len(), 1);
    assert_eq!(
        controller.ui.sources.folders.rows[0].path,
        PathBuf::from("drums")
    );
}

#[test]
fn toggle_show_all_folders_keeps_previous_rows_while_projection_is_pending() {
    let (mut controller, source) = nested_folder_controller();

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.toggle_show_all_folders();
        let request_id = pending_projection_request_id(&controller);

        assert!(
            controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Upper)
                .projecting
        );
        assert!(
            controller
                .ui
                .sources
                .folders
                .rows
                .iter()
                .all(|row| row.path != Path::new("extra-empty"))
        );

        controller.apply_background_job_message_for_tests(JobMessage::FolderProjected(
            projection_result(
                &controller,
                &source,
                request_id,
                vec![
                    root_row(true),
                    row("drums", 1, true, true),
                    row("drums/kicks", 2, false, false),
                    row("extra-empty", 1, false, false),
                ],
                Some(1),
            ),
        ));
    });

    assert!(controller.ui.sources.folders.show_all_folders);
    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .any(|row| row.path == Path::new("extra-empty"))
    );
}
