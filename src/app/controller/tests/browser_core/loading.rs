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
fn label_cache_updates_renamed_slot_without_clearing_cache() {
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

    assert_eq!(
        controller.ui_cache.browser.labels.get(&source.id).cloned(),
        Some(
            crate::app::controller::state::cache::BrowserLabelCacheEntry {
                path_fingerprint: controller.browser_search_path_fingerprint(),
                labels: vec![String::from("renamed"), String::new()],
            }
        )
    );
}

#[test]
fn page_zero_reload_refreshes_same_length_label_cache() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("alpha.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("beta.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert_eq!(controller.wav_label(0).as_deref(), Some("alpha"));
    assert_eq!(controller.wav_label(1).as_deref(), Some("beta"));

    controller.apply_wav_entries_with_params(crate::app::controller::ApplyWavEntriesParams {
        entries: vec![
            sample_entry("beta.wav", crate::sample_sources::Rating::NEUTRAL),
            sample_entry("alpha.wav", crate::sample_sources::Rating::NEUTRAL),
        ],
        total: 2,
        page_size: 2,
        page_index: 0,
        from_cache: false,
        source_id: Some(source.id.clone()),
        elapsed: None,
    });

    assert_eq!(controller.wav_label(0).as_deref(), Some("beta"));
    assert_eq!(controller.wav_label(1).as_deref(), Some("alpha"));
    assert_eq!(
        controller
            .ui_cache
            .browser
            .labels
            .get(&source.id)
            .map(|cache| cache.path_fingerprint),
        Some(controller.browser_search_path_fingerprint())
    );
}
