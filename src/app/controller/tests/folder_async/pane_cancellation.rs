use super::*;

#[test]
fn clearing_pane_cancels_pending_projection_completion() {
    let (mut controller, source) = nested_folder_controller();
    let drums_index = folder_row_index(&controller, "drums");

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.toggle_folder_expanded(drums_index);
        let request_id = pending_projection_request_id(&controller);
        controller.clear_folder_projection_state(crate::app::state::FolderPaneId::Upper);

        assert!(
            !controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Upper)
                .projecting
        );
        assert!(
            controller
                .runtime
                .source_lane
                .folder_projection
                .pending_for_tests(crate::app::state::FolderPaneId::Upper)
                .is_none()
        );

        controller.apply_background_job_message_for_tests(JobMessage::FolderProjected(
            projection_result(
                &controller,
                &source,
                request_id,
                vec![root_row(false), row("stale", 1, false, false)],
                Some(1),
            ),
        ));
    });

    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != Path::new("stale"))
    );
}

#[test]
fn clearing_all_panes_cancels_pending_projection_completions() {
    let (mut controller, source) = nested_folder_controller();
    let drums_index = folder_row_index(&controller, "drums");

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.toggle_folder_expanded(drums_index);
        let request_id = pending_projection_request_id(&controller);
        controller.clear_all_folder_projection_state();

        assert!(
            !controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Upper)
                .projecting
        );
        assert!(
            !controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Lower)
                .projecting
        );

        controller.apply_background_job_message_for_tests(JobMessage::FolderProjected(
            projection_result(
                &controller,
                &source,
                request_id,
                vec![root_row(false), row("stale-all", 1, false, false)],
                Some(1),
            ),
        ));
    });

    assert!(
        controller
            .ui
            .sources
            .folders
            .rows
            .iter()
            .all(|row| row.path != Path::new("stale-all"))
    );
}
