use super::*;

#[test]
fn focus_and_selection_patch_rows_immediately_without_queueing_projection() {
    let (mut controller, _source) = nested_folder_controller();
    let drums_index = folder_row_index(&controller, "drums");

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.focus_folder_row(drums_index);

        assert_eq!(controller.ui.sources.folders.focused, Some(drums_index));
        assert!(
            controller
                .runtime
                .source_lane
                .folder_projection
                .pending_for_tests(crate::app::state::FolderPaneId::Upper)
                .is_none()
        );
        assert!(
            !controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Upper)
                .projecting
        );

        controller.replace_folder_selection(drums_index);

        assert!(controller.ui.sources.folders.rows[drums_index].selected);
        assert!(
            controller
                .runtime
                .source_lane
                .folder_projection
                .pending_for_tests(crate::app::state::FolderPaneId::Upper)
                .is_none()
        );
    });
}
