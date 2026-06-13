use super::*;
#[test]
fn startup_active_source_hydration_defers_follow_up_work_after_first_paint() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();

    let hydrated_entries = vec![
        sample_entry("folder/kick.wav", Rating::KEEP_1),
        sample_entry("snare.wav", Rating::NEUTRAL),
    ];
    std::fs::create_dir_all(sources[1].root.join("folder")).unwrap();
    cache_source_entries(&mut controller, &sources[1], hydrated_entries.clone());

    with_folder_projection_async_enabled_for_tests(true, || {
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
            let mut result = hydration_result(
                &controller,
                &sources[1],
                request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                hydrated_entries.clone(),
                true,
            );
            if let Ok(snapshot) = result.result.as_mut() {
                snapshot.deferred_follow_up_work = true;
            }
            controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(result));
        });
    });

    assert_eq!(controller.selected_source_id(), Some(sources[1].id.clone()));
    assert_eq!(controller.wav_entries.total, 2);
    assert_eq!(visible_indices(&controller), vec![0, 1]);
    assert!(
        controller
            .runtime
            .source_lane
            .folder_projection
            .is_pending(FolderPaneId::Upper)
    );
    assert!(
        controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Upper)
            .projecting
    );
    assert!(
        controller
            .runtime
            .browser
            .pending_feature_cache_refresh
            .is_some()
    );
    assert!(
        !controller
            .ui_cache
            .browser
            .features
            .contains_key(&sources[1].id)
    );
}
