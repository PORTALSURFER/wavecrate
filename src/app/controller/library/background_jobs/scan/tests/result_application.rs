use super::*;

#[test]
fn changed_scan_refreshes_selected_source_without_enqueuing_follow_up_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("kick.wav");
    write_test_wav(&wav_path, &[0.1, -0.1, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("kick.wav"))
        .expect("sample db entry");
    controller.ui_cache.browser.features.insert(
        source.id.clone(),
        FeatureCache {
            key: FeatureCacheKey::default(),
            rows: Vec::new().into(),
        },
    );
    controller.ui_cache.browser.durations.insert(
        source.id.clone(),
        HashMap::from([(PathBuf::from("kick.wav"), 1.25)]),
    );
    controller.show_status_progress(ProgressTaskKind::Scan, "Scanning source", 0, true);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Manual,
            Ok(ScanStats {
                added: 1,
                updated: 0,
                missing: 0,
                changed_samples: vec![changed_sample("kick.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
    assert!(
        !controller
            .ui_cache
            .browser
            .features
            .contains_key(&source.id)
    );
    assert!(controller.wav_entries.source_id.as_ref() == Some(&source.id));
    assert_eq!(controller.wav_entries.total, 1);
    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn rename_only_quick_scan_applies_anchored_browser_delta_without_wav_reload() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    controller.set_wav_entries_for_tests(vec![
        crate::app::controller::test_support::sample_entry("alpha.wav", Rating::NEUTRAL),
        crate::app::controller::test_support::sample_entry("old.wav", Rating::KEEP_1),
        crate::app::controller::test_support::sample_entry("zulu.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.select_wav_by_index(1);
    controller.ui.browser.selection.selection_anchor_visible = Some(1);
    controller.ui.browser.viewport.view_window_start = 1;
    let visible_revision = controller.ui.browser.viewport.visible_rows_revision;

    let db = controller.database_for(&source).expect("source db");
    db.upsert_file(Path::new("renamed.wav"), 8, 42)
        .expect("upsert renamed row");
    db.set_tag(Path::new("renamed.wav"), Rating::KEEP_1)
        .expect("set tag");
    drop(db);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats {
                updated: 1,
                renames_reconciled: 1,
                renamed_samples: vec![renamed_sample("old.wav", "renamed.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    assert_eq!(controller.wav_entries.total, 3);
    assert!(
        controller
            .wav_index_for_path(Path::new("old.wav"))
            .is_none()
    );
    assert_eq!(
        controller.wav_index_for_path(Path::new("renamed.wav")),
        Some(1)
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("renamed.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(controller.ui.browser.viewport.view_window_start, 1);
    assert_eq!(
        controller.ui.browser.viewport.visible_rows_revision,
        visible_revision
    );
    assert_no_analysis_message(&mut controller);
}

#[test]
fn small_updated_quick_scan_patches_cached_entry_without_wav_reload() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    controller.set_wav_entries_for_tests(vec![crate::app::controller::test_support::sample_entry(
        "kick.wav",
        Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    let db = controller.database_for(&source).expect("source db");
    db.upsert_file(Path::new("kick.wav"), 8, 42)
        .expect("upsert updated row");
    drop(db);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats {
                updated: 1,
                updated_samples: vec![updated_sample("kick.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    let index = controller
        .wav_index_for_path(Path::new("kick.wav"))
        .expect("updated row remains loaded");
    let entry = controller.wav_entries.entry(index).expect("entry");
    assert_eq!(entry.file_size, 8);
    assert_eq!(entry.modified_ns, 42);
    assert_eq!(controller.wav_entries.total, 1);
    assert_no_analysis_message(&mut controller);
}
