use super::*;
#[test]
fn inactive_pane_source_hydration_keeps_active_browser_state_stable() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();
    controller.select_wav_by_index(0);

    let inactive_entries = vec![sample_entry("drums/kick.wav", Rating::NEUTRAL)];
    std::fs::create_dir_all(sources[1].root.join("drums")).unwrap();

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index_in_pane(FolderPaneId::Lower, 1);

        assert_eq!(controller.selected_source_id(), Some(sources[0].id.clone()));
        assert_eq!(visible_indices(&controller), vec![0]);
        assert_eq!(
            controller.sample_view.wav.selected_wav,
            Some(PathBuf::from("alpha.wav"))
        );
        assert!(
            controller
                .ui
                .sources
                .folder_pane(FolderPaneId::Lower)
                .loading
        );
        assert!(
            controller
                .ui
                .sources
                .folder_pane(FolderPaneId::Lower)
                .browser
                .rows
                .is_empty()
        );

        let request_id = controller
            .runtime
            .source_lane
            .hydration
            .pending_inactive
            .as_ref()
            .expect("inactive pane hydration")
            .request_id;
        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[1],
                request_id,
                FolderPaneId::Lower,
                SourceHydrationKind::InactivePane,
                inactive_entries.clone(),
                true,
            ),
        ));
    });

    assert_eq!(controller.selected_source_id(), Some(sources[0].id.clone()));
    assert_eq!(
        controller.sample_view.wav.selected_wav,
        Some(PathBuf::from("alpha.wav"))
    );
    assert_eq!(visible_indices(&controller), vec![0]);
    assert!(
        !controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Lower)
            .loading
    );
    assert_eq!(
        controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Lower)
            .browser
            .rows
            .len(),
        2
    );
}

#[test]
fn async_source_hydration_keeps_loading_until_async_browser_projection_applies() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();
    let hydrated_entries = vec![sample_entry("beta.wav", Rating::NEUTRAL)];

    with_source_hydration_async_enabled_for_tests(true, || {
        with_browser_async_pipeline_enabled_for_tests(true, || {
            controller.select_source_by_index(1);
            let request_id = controller
                .runtime
                .source_lane
                .hydration
                .pending_active
                .as_ref()
                .expect("pending hydration")
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

            assert!(controller.ui.browser.search.source_loading);
            assert!(controller.ui.browser.search.search_busy);

            let search_request_id = controller
                .runtime
                .source_lane
                .hydration
                .pending_active
                .as_ref()
                .and_then(|pending| pending.search_request_id)
                .expect("queued browser search request");
            let visible = crate::app::state::VisibleRows::List(vec![0usize].into());
            controller.apply_background_job_message_for_tests(JobMessage::BrowserSearchFinished(
                crate::app::controller::jobs::SearchResult {
                    request_id: search_request_id,
                    source_id: sources[1].id.clone(),
                    query: String::new(),
                    visible,
                    trash: std::sync::Arc::from([]),
                    neutral: std::sync::Arc::from([0usize]),
                    keep: std::sync::Arc::from([]),
                    scores: std::sync::Arc::from([]),
                },
            ));
        });
    });

    assert!(!controller.ui.browser.search.source_loading);
    assert!(!controller.ui.browser.search.search_busy);
    assert_eq!(visible_indices(&controller), vec![0]);
}
