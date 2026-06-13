use super::*;

#[test]
fn expand_toggle_queues_async_projection_and_drops_stale_results() {
    let (mut controller, source) = nested_folder_controller();
    let drums_index = folder_row_index(&controller, "drums");

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.toggle_folder_expanded(drums_index);
        let first_request_id = pending_projection_request_id(&controller);
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
                .any(|row| row.path == Path::new("drums/kicks"))
        );

        controller.toggle_folder_expanded(drums_index);
        let second_request_id = pending_projection_request_id(&controller);
        assert_ne!(first_request_id, second_request_id);

        controller.apply_background_job_message_for_tests(JobMessage::FolderProjected(
            projection_result(
                &controller,
                &source,
                first_request_id,
                vec![root_row(true), row("drums", 1, true, false)],
                Some(1),
            ),
        ));
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
                vec![
                    root_row(true),
                    row("drums", 1, true, true),
                    row("drums/kicks", 2, false, false),
                ],
                Some(1),
            ),
        ));
    });

    assert!(
        !controller
            .ui
            .sources
            .folder_pane(crate::app::state::FolderPaneId::Upper)
            .projecting
    );
    assert_eq!(controller.ui.sources.folders.rows.len(), 3);
    assert!(
        controller.ui.sources.folders.rows[1].expanded,
        "final matching result should apply"
    );
}
