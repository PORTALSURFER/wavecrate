use super::*;
#[test]
fn update_entry_metadata_refreshes_partitions_without_invalidating_base_snapshot() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);
    let base_fingerprint = controller
        .ui_cache
        .browser
        .pipeline
        .base_fingerprint
        .clone();
    let updated = search_entry("neutral.wav", Rating::TRASH_1, None);

    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .update_entry_metadata(0, &updated)
    );

    assert_eq!(
        controller.ui_cache.browser.pipeline.base_fingerprint,
        base_fingerprint
    );
    assert_eq!(controller.ui_cache.browser.pipeline.trash_rows, vec![0]);
    assert_eq!(controller.ui_cache.browser.pipeline.keep_rows, vec![1]);
}

#[test]
fn update_entry_metadata_keeps_folder_acceptance_cache_for_same_path_edits() {
    let entries = vec![
        search_entry("drums/kick.wav", Rating::NEUTRAL, None),
        search_entry("hits/snare.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    let selection = BTreeSet::from([PathBuf::from("drums")]);
    let folder_hash = crate::app::controller::library::source_folders::folder_filter_fingerprint(
        Some(&selection),
        None,
        FolderFileScopeMode::DirectOnly,
    );

    ensure_base_stage(&mut controller);
    ensure_folder_acceptance_stage(
        &mut controller,
        Some(&selection),
        None,
        FolderFileScopeMode::DirectOnly,
        folder_hash,
        true,
    );
    let folder_accepts_fingerprint = controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint;
    let folder_accepts_by_index = controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_by_index
        .clone();

    let updated = search_entry("drums/kick.wav", Rating::TRASH_1, None);
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .update_entry_metadata(0, &updated)
    );

    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .folder_accepts_fingerprint,
        folder_accepts_fingerprint
    );
    assert_eq!(
        controller.ui_cache.browser.pipeline.folder_accepts_by_index,
        folder_accepts_by_index
    );
}

#[test]
fn update_entry_metadata_invalidates_filtered_stage_without_rebuilding_base_rows() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.filter = TriageFlagFilter::Keep;

    let _ = build_visible_rows(&mut controller, None, None);
    let base_rows = controller.ui_cache.browser.pipeline.base_rows.clone();
    let base_fingerprint = controller
        .ui_cache
        .browser
        .pipeline
        .base_fingerprint
        .clone();
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .filtered_fingerprint
            .is_some()
    );

    let updated = search_entry("neutral.wav", Rating::KEEP_3, None);
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .update_entry_metadata(0, &updated)
    );

    assert_eq!(controller.ui_cache.browser.pipeline.base_rows, base_rows);
    assert_eq!(
        controller.ui_cache.browser.pipeline.base_fingerprint,
        base_fingerprint
    );
    assert_eq!(controller.ui_cache.browser.pipeline.keep_rows, vec![0, 1]);
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .filtered_fingerprint
            .is_none()
    );
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .sorted_fingerprint
            .is_none()
    );
}
