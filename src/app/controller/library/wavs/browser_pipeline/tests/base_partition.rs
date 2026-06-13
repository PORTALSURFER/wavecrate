use super::*;
#[test]
fn base_stage_partitions_rows_by_triage_bucket() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("trash.wav", Rating::TRASH_1, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);

    assert_eq!(
        controller.ui_cache.browser.pipeline.base_rows,
        vec![0, 1, 2]
    );
    assert_eq!(controller.ui_cache.browser.pipeline.trash_rows, vec![1]);
    assert_eq!(controller.ui_cache.browser.pipeline.neutral_rows, vec![0]);
    assert_eq!(controller.ui_cache.browser.pipeline.keep_rows, vec![2]);
}

#[test]
fn base_stage_reuses_cached_fingerprint_without_rechecking_db_revision() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);
    let first_fingerprint = controller
        .ui_cache
        .browser
        .pipeline
        .base_fingerprint
        .clone();
    controller.cache.db.clear();
    let invalid_root = source.root.join("missing-after-cache");
    let selected = controller
        .library
        .sources
        .iter_mut()
        .find(|candidate| candidate.id == source.id)
        .expect("selected source");
    selected.root = invalid_root;

    ensure_base_stage(&mut controller);

    assert_eq!(
        controller.ui_cache.browser.pipeline.base_fingerprint,
        first_fingerprint
    );
    assert_eq!(controller.ui_cache.browser.pipeline.base_rows, vec![0, 1]);
}

#[test]
fn base_stage_rebuilds_after_same_path_tag_updates() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);
    crate::app::controller::library::wavs::selection_ops::set_sample_tag_for_source(
        &mut controller,
        &source,
        Path::new("neutral.wav"),
        Rating::TRASH_1,
        false,
    )
    .expect("update tag");
    ensure_base_stage(&mut controller);

    assert_eq!(controller.ui_cache.browser.pipeline.trash_rows, vec![0]);
    assert_eq!(controller.ui_cache.browser.pipeline.keep_rows, vec![1]);
}
