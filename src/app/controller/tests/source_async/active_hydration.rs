use super::*;
#[test]
fn selecting_cached_source_clears_browser_until_async_hydration_applies() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("alpha.wav", Rating::NEUTRAL),
        sample_entry("beta.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_browser_lists();

    let cached_entries = vec![
        sample_entry("folder/kick.wav", Rating::KEEP_1),
        sample_entry("snare.wav", Rating::NEUTRAL),
    ];
    std::fs::create_dir_all(sources[1].root.join("folder")).unwrap();
    cache_source_entries(&mut controller, &sources[1], cached_entries.clone());

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index(1);

        assert_eq!(controller.ui.sources.selected, Some(1));
        assert_eq!(
            controller.ui.sources.loading_source_id,
            Some(sources[1].id.clone())
        );
        assert!(controller.ui.browser.search.source_loading);
        assert!(!controller.ui.browser.search.search_busy);
        assert!(visible_indices(&controller).is_empty());
        assert!(controller.ui.sources.folders.rows.is_empty());

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
                cached_entries.clone(),
                true,
            ),
        ));
    });

    assert_eq!(visible_indices(&controller), vec![0, 1]);
    assert_eq!(
        controller.sample_view.wav.selected_wav,
        Some(PathBuf::from("folder/kick.wav"))
    );
    assert!(!controller.ui.browser.search.source_loading);
    assert!(
        !controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Upper)
            .loading
    );
}

#[test]
fn stale_uncached_source_hydration_result_is_dropped() {
    let (mut controller, sources) =
        build_controller_with_sources(&["source-a", "source-b", "source-c"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();

    let source_b_entries = vec![sample_entry("drums/kick.wav", Rating::NEUTRAL)];
    let source_c_entries = vec![sample_entry("vox.wav", Rating::KEEP_1)];
    std::fs::create_dir_all(sources[1].root.join("drums")).unwrap();
    for entry in &source_b_entries {
        upsert_source_db_entry(&mut controller, &sources[1], entry);
    }
    for entry in &source_c_entries {
        upsert_source_db_entry(&mut controller, &sources[2], entry);
    }

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index(1);
        let first_request_id = controller
            .runtime
            .source_lane
            .hydration
            .pending_active
            .as_ref()
            .expect("first pending hydration")
            .request_id;

        controller.select_source_by_index(2);
        let second_request_id = controller
            .runtime
            .source_lane
            .hydration
            .pending_active
            .as_ref()
            .expect("second pending hydration")
            .request_id;

        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[1],
                first_request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                source_b_entries.clone(),
                false,
            ),
        ));

        assert!(visible_indices(&controller).is_empty());
        assert_eq!(controller.selected_source_id(), Some(sources[2].id.clone()));
        assert!(controller.ui.browser.search.source_loading);

        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[2],
                second_request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                source_c_entries.clone(),
                false,
            ),
        ));
    });

    assert_eq!(visible_indices(&controller), vec![0]);
    assert_eq!(
        controller.sample_view.wav.selected_wav,
        Some(PathBuf::from("vox.wav"))
    );
    assert!(!controller.ui.browser.search.source_loading);
}
