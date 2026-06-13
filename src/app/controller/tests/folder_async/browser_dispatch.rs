use super::*;

#[test]
fn folder_selection_dispatches_browser_search_without_marking_folder_projection_busy() {
    let (mut controller, _source) = nested_folder_controller();
    let drums_index = folder_row_index(&controller, "drums");

    with_browser_async_pipeline_enabled_for_tests(true, || {
        with_folder_projection_async_enabled_for_tests(true, || {
            controller.replace_folder_selection(drums_index);

            assert!(controller.ui.browser.search.search_busy);
            assert!(
                !controller
                    .ui
                    .sources
                    .folder_pane(crate::app::state::FolderPaneId::Upper)
                    .projecting
            );
        });
    });
}
