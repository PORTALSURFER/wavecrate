use super::*;
#[test]
fn deferred_source_db_maintenance_full_reload_refreshes_selected_source() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a"]);
    let source = sources[0].clone();
    let initial_entries = vec![sample_entry("alpha.wav", Rating::NEUTRAL)];
    controller.set_wav_entries_for_tests(initial_entries.clone());
    cache_source_entries(&mut controller, &source, initial_entries.clone());
    controller.rebuild_browser_lists();
    assert_eq!(visible_indices(&controller), vec![0]);

    let refreshed_entries = vec![
        sample_entry("alpha.wav", Rating::NEUTRAL),
        sample_entry("beta.wav", Rating::KEEP_1),
    ];
    for entry in &refreshed_entries {
        upsert_source_db_entry(&mut controller, &source, entry);
    }

    controller.apply_background_job_message_for_tests(maintenance_result(
        &source,
        SourceDbMaintenanceRefresh::FullSourceReload,
        0,
    ));

    assert_eq!(controller.selected_source_id(), Some(source.id.clone()));
    assert_eq!(controller.wav_entries.total, 2);
    assert_eq!(visible_indices(&controller), vec![0, 1]);
    assert_no_analysis_message(&mut controller);
}

#[test]
fn deferred_source_db_maintenance_file_op_reconcile_keeps_wav_cache_stable() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a"]);
    let source = sources[0].clone();
    let initial_entries = vec![sample_entry("alpha.wav", Rating::NEUTRAL)];
    controller.set_wav_entries_for_tests(initial_entries.clone());
    cache_source_entries(&mut controller, &source, initial_entries.clone());
    controller.rebuild_browser_lists();
    assert_eq!(visible_indices(&controller), vec![0]);

    let refreshed_entries = vec![
        sample_entry("alpha.wav", Rating::NEUTRAL),
        sample_entry("beta.wav", Rating::KEEP_1),
    ];
    for entry in &refreshed_entries {
        upsert_source_db_entry(&mut controller, &source, entry);
    }

    controller.apply_background_job_message_for_tests(maintenance_result(
        &source,
        SourceDbMaintenanceRefresh::FileOpReconcile,
        0,
    ));

    assert_eq!(controller.selected_source_id(), Some(source.id.clone()));
    assert_eq!(controller.wav_entries.total, 1);
    assert_eq!(visible_indices(&controller), vec![0]);
    assert!(controller.cache.wav.entries.contains_key(&source.id));
    assert_no_analysis_message(&mut controller);
}
