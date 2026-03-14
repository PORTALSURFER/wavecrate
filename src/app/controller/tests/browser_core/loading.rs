use super::*;

#[test]
fn missing_source_is_marked_during_load() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::remove_dir_all(&source.root).unwrap();
    controller.queue_wav_load();
    controller.poll_background_jobs();
    assert_eq!(controller.library.sources.len(), 1);
    assert!(controller.library.missing.sources.contains(&source.id));
    assert!(
        controller
            .ui
            .sources
            .rows
            .first()
            .is_some_and(|row| row.missing)
    );
}

#[test]
fn label_cache_builds_on_first_lookup() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert!(!controller.ui_cache.browser.labels.contains_key(&source.id));
    let label = controller.wav_label(1).unwrap();
    assert_eq!(label, "b");
    assert!(controller.ui_cache.browser.labels.contains_key(&source.id));
}

#[test]
fn label_cache_clears_after_rename() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("b.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert_eq!(controller.wav_label(0).unwrap(), "a");
    assert!(controller.ui_cache.browser.labels.contains_key(&source.id));

    controller.update_cached_entry(
        &source,
        Path::new("a.wav"),
        sample_entry("renamed.wav", crate::sample_sources::Rating::NEUTRAL),
    );

    assert!(!controller.ui_cache.browser.labels.contains_key(&source.id));
}
