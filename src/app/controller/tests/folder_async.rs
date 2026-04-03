use super::super::jobs::{FolderProjectionResult, FolderProjectionSnapshot, JobMessage};
use super::super::library::source_folders::with_folder_projection_async_enabled_for_tests;
use super::super::library::wavs::with_browser_async_pipeline_enabled_for_tests;
use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use super::super::*;
use crate::app::controller::library::source_folders::{FolderProjectionView, FolderTreeSnapshot};
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn folder_row_index(controller: &AppController, path: &str) -> usize {
    controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from(path))
        .unwrap_or_else(|| panic!("missing folder row for {path}"))
}

fn visible_folder_paths(controller: &AppController) -> Vec<PathBuf> {
    controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect()
}

fn nested_folder_controller() -> (AppController, SampleSource) {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.assign_source_to_folder_pane(
        crate::app::state::FolderPaneId::Upper,
        Some(source.id.clone()),
    );
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller
        .selection_state
        .ctx
        .last_selected_browsable_source = Some(source.id.clone());
    let nested = source.root.join("drums").join("kicks");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir_all(source.root.join("extra-empty")).unwrap();
    write_test_wav(&nested.join("tight.wav"), &[0.2, -0.2]);
    controller
        .set_wav_entries_for_tests(vec![sample_entry("drums/kicks/tight.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.refresh_folder_browser_for_tests();
    (controller, source)
}

fn pending_projection_request_id(controller: &AppController) -> u64 {
    controller
        .runtime
        .pending_folder_projections
        .get(&crate::app::state::FolderPaneId::Upper)
        .expect("pending folder projection")
        .request_id
}

fn projection_result(
    controller: &AppController,
    source: &SampleSource,
    request_id: u64,
    rows: Vec<crate::app::state::FolderRowView>,
    focused: Option<usize>,
) -> FolderProjectionResult {
    let pane = crate::app::state::FolderPaneId::Upper;
    let key = crate::app::controller::state::cache::FolderBrowserCacheKey {
        pane,
        source_id: source.id.clone(),
    };
    let model = controller
        .ui_cache
        .folders
        .models
        .get(&key)
        .cloned()
        .expect("folder model");
    let tree = controller
        .ui_cache
        .folders
        .snapshots
        .get(&key)
        .cloned()
        .unwrap_or_else(|| FolderTreeSnapshot::from_available(&model.available));
    let flattened_view =
        model.file_scope_mode == crate::app::state::FolderFileScopeMode::AllDescendants;
    let search_query = model.search_query.clone();
    let show_all_folders = model.show_all_folders;
    FolderProjectionResult {
        request_id,
        pane,
        source_id: source.id.clone(),
        elapsed: Duration::from_millis(5),
        snapshot: FolderProjectionSnapshot {
            model,
            tree,
            view: FolderProjectionView {
                rows,
                focused,
                scroll_to: focused,
                search_query,
                show_all_folders,
                flattened_view,
            },
        },
    }
}

fn root_row(has_children: bool) -> crate::app::state::FolderRowView {
    crate::app::state::FolderRowView {
        path: PathBuf::new(),
        name: String::from("Root"),
        depth: 0,
        has_children,
        expanded: true,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: true,
        file_scope_mode: Some(crate::app::state::FolderFileScopeMode::DirectOnly),
    }
}

fn row(
    path: &str,
    depth: usize,
    has_children: bool,
    expanded: bool,
) -> crate::app::state::FolderRowView {
    let path_buf = PathBuf::from(path);
    crate::app::state::FolderRowView {
        name: path_buf
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.to_string()),
        path: path_buf,
        depth,
        has_children,
        expanded,
        selected: false,
        negated: false,
        hotkey: None,
        is_root: false,
        file_scope_mode: None,
    }
}

#[test]
fn focus_and_selection_patch_rows_immediately_without_queueing_projection() {
    let (mut controller, _source) = nested_folder_controller();
    let drums_index = folder_row_index(&controller, "drums");

    with_folder_projection_async_enabled_for_tests(true, || {
        controller.focus_folder_row(drums_index);

        assert_eq!(controller.ui.sources.folders.focused, Some(drums_index));
        assert!(controller.runtime.pending_folder_projections.is_empty());
        assert!(
            !controller
                .ui
                .sources
                .folder_pane(crate::app::state::FolderPaneId::Upper)
                .projecting
        );

        controller.replace_folder_selection(drums_index);

        assert!(controller.ui.sources.folders.rows[drums_index].selected);
        assert!(controller.runtime.pending_folder_projections.is_empty());
    });
}

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
