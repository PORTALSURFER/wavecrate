use super::*;

#[test]
fn auto_rename_allows_paths_with_pending_metadata_mutations() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: 1,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                blocks_file_mutation: true,
                rollback: Vec::new(),
                refresh_browser_projection: false,
            },
        );

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(!source.root.join("raw.wav").exists());
    assert!(source.root.join("portal_SS.wav").exists());
}

#[test]
fn auto_rename_ignores_pending_analysis_only_metadata_mutations() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: 1,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                blocks_file_mutation: false,
                rollback: Vec::new(),
                refresh_browser_projection: false,
            },
        );

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(!source.root.join("raw.wav").exists());
    assert!(source.root.join("portal_SS.wav").exists());
}
