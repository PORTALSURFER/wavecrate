use super::*;

#[test]
fn tagging_keeps_selection_on_same_sample() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
}

#[test]
fn left_tagging_from_keep_untags_then_trashes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::KEEP_1),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.tag_selected_left();
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );

    controller.tag_selected_left();
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
}

#[test]
fn tagging_under_filter_advances_focus_to_next_visible() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("four.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.set_browser_filter(TriageFlagFilter::Untagged);

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(visible_indices(&controller), vec![0, 2, 3]);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
}

#[test]
fn tagging_under_search_filter_updates_hidden_selected_paths() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.focus_browser_row_only(0);

    controller.set_browser_search(String::from("one"));
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    let one_index = controller.wav_index_for_path(Path::new("one.wav")).unwrap();
    let two_index = controller.wav_index_for_path(Path::new("two.wav")).unwrap();
    let three_index = controller
        .wav_index_for_path(Path::new("three.wav"))
        .unwrap();

    assert_eq!(
        controller.wav_entry(one_index).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
    assert_eq!(
        controller.wav_entry(two_index).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
    assert_eq!(
        controller.wav_entry(three_index).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn browser_tag_sidebar_mutation_uses_selected_visible_target_snapshot_fallback() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.ui.browser.selection.selected_visible = Some(1);
    controller.ui.browser.selection.last_focused_path = None;
    controller.ui.browser.selection.selected_paths.clear();

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("selected-visible fallback should resolve one target");

    assert!(!controller.wav_entry(0).unwrap().looped);
    assert!(controller.wav_entry(1).unwrap().looped);
}

#[test]
fn browser_tag_sidebar_common_tag_assigns_normal_tag_catalog_entry() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.focus_browser_row_only(0);

    controller
        .apply_browser_tag_sidebar_sound_type(Some(crate::sample_sources::SampleSoundType::Kick))
        .expect("common tag should assign");

    let tags = controller
        .database_for(&source)
        .unwrap()
        .tags_for_path(Path::new("one.wav"))
        .unwrap();
    assert_eq!(tag_labels(tags), vec!["kick"]);
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .sound_type_for_path(Path::new("one.wav"))
            .unwrap(),
        None
    );
}

#[test]
fn browser_tag_sidebar_typed_input_resolves_existing_fuzzy_tag() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller
        .database_for(&source)
        .unwrap()
        .assign_tag_to_path(Path::new("one.wav"), "Deep Kick")
        .unwrap();
    controller.focus_browser_row_only(1);
    controller.set_browser_tag_sidebar_input(String::from("kick"));

    controller
        .commit_browser_tag_sidebar_input()
        .expect("typed tag should resolve and assign");

    let tags = controller
        .database_for(&source)
        .unwrap()
        .tags_for_path(Path::new("two.wav"))
        .unwrap();
    assert_eq!(tag_labels(tags), vec!["Deep Kick"]);
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
}

#[test]
fn browser_tag_sidebar_typed_input_creates_reusable_normal_tag() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.set_browser_tag_sidebar_input(String::from("  Vintage   FX "));

    controller
        .commit_browser_tag_sidebar_input()
        .expect("typed tag should create and assign");

    let db = controller.database_for(&source).unwrap();
    assert_eq!(
        tag_labels(db.tags_for_path(Path::new("one.wav")).unwrap()),
        vec!["Vintage FX"]
    );
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
    controller.focus_browser_row_only(1);
    controller.set_browser_tag_sidebar_input(String::from("vintage"));
    controller
        .commit_browser_tag_sidebar_input()
        .expect("created tag should be reusable by search");
    assert_eq!(
        tag_labels(db.tags_for_path(Path::new("two.wav")).unwrap()),
        vec!["Vintage FX"]
    );
}

#[test]
fn browser_tag_sidebar_multi_selection_tracks_mixed_and_removes_normal_tags() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller
        .database_for(&source)
        .unwrap()
        .assign_tag_to_path(Path::new("one.wav"), "kick")
        .unwrap();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let paths = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];

    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "kick")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::Mixed
    );

    controller
        .apply_browser_tag_sidebar_normal_tag("kick")
        .expect("assignment should apply to every selected path");
    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "kick")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::On
    );
    controller
        .remove_browser_tag_sidebar_normal_tag("kick")
        .expect("removal should apply to every selected path");

    let db = controller.database_for(&source).unwrap();
    assert!(db.tags_for_path(Path::new("one.wav")).unwrap().is_empty());
    assert!(db.tags_for_path(Path::new("two.wav")).unwrap().is_empty());
}

#[test]
fn browser_tag_sidebar_multi_selection_queues_one_normal_tag_metadata_batch() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    let paths = vec![
        PathBuf::from("one.wav"),
        PathBuf::from("two.wav"),
        PathBuf::from("three.wav"),
    ];
    controller.set_browser_selected_paths(paths.clone());

    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("assignment should batch selected paths");

    let samples = crate::app::controller::batch_latency::snapshot();
    let queue_samples = samples
        .iter()
        .filter(|sample| {
            sample.phase
                == crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue
        })
        .collect::<Vec<_>>();
    assert_eq!(queue_samples.len(), 1, "{samples:#?}");
    assert_eq!(queue_samples[0].item_count, paths.len());
    assert_eq!(queue_samples[0].detail_count, paths.len());
    for path in &paths {
        let index = controller.wav_index_for_path(path).unwrap();
        assert_eq!(
            controller.wav_entry(index).unwrap().normal_tags,
            vec![String::from("Vintage FX")]
        );
    }
}

#[test]
fn browser_tag_sidebar_multi_selection_queues_one_looped_metadata_batch() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    let paths = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];
    controller.set_browser_selected_paths(paths.clone());

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("loop marker should batch selected paths");

    let samples = crate::app::controller::batch_latency::snapshot();
    let queue_samples = samples
        .iter()
        .filter(|sample| {
            sample.phase
                == crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue
        })
        .collect::<Vec<_>>();
    assert_eq!(queue_samples.len(), 1, "{samples:#?}");
    assert_eq!(queue_samples[0].item_count, paths.len());
    assert_eq!(queue_samples[0].detail_count, paths.len());
    for path in &paths {
        let index = controller.wav_index_for_path(path).unwrap();
        assert!(controller.wav_entry(index).unwrap().looped);
    }
}

#[test]
fn rating_write_require_present_rejects_missing_path_without_queueing() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result = controller.set_sample_tag_for_source(
        &source,
        Path::new("missing.wav"),
        crate::sample_sources::Rating::KEEP_1,
        true,
    );

    assert_eq!(result, Err(String::from("Sample not found")));
    assert!(metadata_queue_samples().is_empty());
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
}

#[test]
fn rating_write_without_require_present_preserves_permissive_missing_path_behavior() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result = controller.set_sample_tag_for_source(
        &source,
        Path::new("missing.wav"),
        crate::sample_sources::Rating::KEEP_1,
        false,
    );

    assert_eq!(result, Ok(()));
    assert_eq!(metadata_queue_samples().len(), 1);
}

#[test]
fn looped_write_require_present_rejects_missing_single_path_without_queueing() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result =
        controller.set_sample_looped_for_source(&source, Path::new("missing.wav"), true, true);

    assert_eq!(result, Err(String::from("Sample not found")));
    assert!(metadata_queue_samples().is_empty());
    assert!(!controller.wav_entry(0).unwrap().looped);
}

#[test]
fn looped_batch_require_present_rejects_missing_path_before_intents_or_cache_updates() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result = controller.set_sample_looped_for_source_batch(
        &source,
        &[PathBuf::from("one.wav"), PathBuf::from("missing.wav")],
        true,
        true,
    );

    assert_eq!(result, Err(String::from("Sample not found")));
    assert!(metadata_queue_samples().is_empty());
    assert!(!controller.wav_entry(0).unwrap().looped);
}

#[test]
fn browser_tag_sidebar_batch_failure_rolls_back_each_normal_tag_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    let paths = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];
    controller.set_browser_selected_paths(paths.clone());
    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("assignment should apply optimistically");
    let request_id = 999;
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id,
                source_id: source.id.clone(),
                paths: paths.iter().cloned().collect(),
                blocks_file_mutation: true,
                rollback: paths
                    .iter()
                    .map(|path| {
                        crate::app::controller::state::runtime::MetadataRollback::NormalTag {
                            relative_path: path.clone(),
                            normalized_text: String::from("vintage fx"),
                            display_label: String::from("Vintage FX"),
                            before_present: false,
                            expected_present: true,
                        }
                    })
                    .collect(),
                refresh_browser_projection: true,
            },
        );

    controller.handle_metadata_mutation_finished_message(
        crate::app::controller::jobs::MetadataMutationResult {
            request_id,
            source_id: source.id.clone(),
            paths: paths.iter().cloned().collect(),
            elapsed: std::time::Duration::ZERO,
            result: Err(String::from("forced metadata failure")),
        },
    );

    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "Vintage FX")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::Off
    );
    for path in &paths {
        let index = controller.wav_index_for_path(path).unwrap();
        assert!(controller.wav_entry(index).unwrap().normal_tags.is_empty());
    }
}

#[test]
fn metadata_rollback_uses_mutation_source_after_source_switch() {
    let (mut controller, source_a) = dummy_controller();
    let source_b_root = source_a.root.parent().unwrap().join("source-b");
    std::fs::create_dir_all(&source_b_root).unwrap();
    let source_b = crate::sample_sources::SampleSource::new(source_b_root);
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.cache_db(&source_a).unwrap();
    controller.cache_db(&source_b).unwrap();

    let mut source_a_entry = sample_entry("shared.wav", crate::sample_sources::Rating::KEEP_1);
    source_a_entry.locked = true;
    controller
        .cache
        .wav
        .insert_page(source_a.id.clone(), 1, 100, 0, vec![source_a_entry.clone()]);

    controller.select_source_by_index(1);
    let mut source_b_entry = sample_entry("shared.wav", crate::sample_sources::Rating::KEEP_3);
    source_b_entry.locked = true;
    controller.set_wav_entries_for_tests(vec![source_b_entry]);
    controller.rebuild_browser_lists();

    let request_id = 8181;
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id,
                source_id: source_a.id.clone(),
                paths: [PathBuf::from("shared.wav")].into_iter().collect(),
                blocks_file_mutation: true,
                rollback: vec![
                    crate::app::controller::state::runtime::MetadataRollback::TagAndLocked {
                        relative_path: PathBuf::from("shared.wav"),
                        before_tag: crate::sample_sources::Rating::NEUTRAL,
                        before_locked: false,
                        expected_tag: crate::sample_sources::Rating::KEEP_1,
                        expected_locked: true,
                    },
                ],
                refresh_browser_projection: true,
            },
        );

    controller.handle_metadata_mutation_finished_message(
        crate::app::controller::jobs::MetadataMutationResult {
            request_id,
            source_id: source_a.id.clone(),
            paths: [PathBuf::from("shared.wav")].into_iter().collect(),
            elapsed: std::time::Duration::ZERO,
            result: Err(String::from("forced metadata failure")),
        },
    );

    let source_a_cache = controller.cache.wav.entries.get(&source_a.id).unwrap();
    let source_a_index = source_a_cache
        .lookup
        .get(Path::new("shared.wav"))
        .copied()
        .unwrap();
    let source_a_cached = source_a_cache.entry(source_a_index).unwrap();
    assert_eq!(source_a_cached.tag, crate::sample_sources::Rating::NEUTRAL);
    assert!(!source_a_cached.locked);

    let source_b_active = controller.wav_entry(0).unwrap();
    assert_eq!(source_b_active.tag, crate::sample_sources::Rating::KEEP_3);
    assert!(source_b_active.locked);
}

#[test]
fn rating_filter_rating_keeps_focus_on_next_visible_item() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("four.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("four.wav"), &[0.0, 0.1]);
    controller.settings.controls.advance_after_rating = true;
    controller.settings.feature_flags.autoplay_selection = false;
    controller.set_browser_rating_filter(0, false);

    controller.focus_browser_row_only(1);
    controller.adjust_selected_rating(1);

    assert_eq!(visible_indices(&controller), vec![0, 2, 3]);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert!(browser_row_is_queued_or_loaded(
        &controller,
        Path::new("three.wav")
    ));
}

fn tag_labels(tags: Vec<crate::sample_sources::db::SourceTag>) -> Vec<String> {
    tags.into_iter().map(|tag| tag.display_label).collect()
}

fn metadata_queue_samples() -> Vec<crate::app::controller::batch_latency::BatchLatencySample> {
    crate::app::controller::batch_latency::snapshot()
        .into_iter()
        .filter(|sample| {
            sample.phase
                == crate::app::controller::batch_latency::BatchLatencyPhase::MetadataMutationQueue
        })
        .collect()
}

#[test]
fn tagging_under_filter_uses_random_focus_in_random_mode() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    controller.settings.controls.advance_after_rating = true;
    controller.set_browser_filter(TriageFlagFilter::Untagged);
    controller.toggle_random_navigation_mode();

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(visible_indices(&controller), vec![0, 2]);
    assert_eq!(controller.history.random_history.entries.len(), 1);
    assert_eq!(controller.history.random_history.cursor, Some(0));
    let Some(selected_visible) = controller.ui.browser.selection.selected_visible else {
        panic!("expected a selected row");
    };
    assert!(selected_visible < controller.visible_browser_len());
    let selected_path = controller
        .sample_view
        .wav
        .selected_wav
        .as_deref()
        .expect("selected replacement row");
    assert!(browser_row_is_queued_or_loaded(&controller, selected_path));
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(selected_path)
    );
    assert!(controller.ui.waveform.image.is_none());
}

#[test]
fn undo_tagging_refocuses_original_sample_under_filter() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.set_browser_filter(TriageFlagFilter::Untagged);

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);
    assert_eq!(visible_indices(&controller), vec![0, 2]);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );

    controller.undo();

    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
    assert_eq!(
        controller.wav_entry(1).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
}

#[test]
fn direct_keep_three_tag_locks_sample_and_blocks_future_tag_changes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "keep3_direct.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller.tag_selected(crate::sample_sources::Rating::KEEP_3);

    let entry = controller
        .wav_entry(0)
        .expect("locked sample should stay loaded");
    assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_3);
    assert!(entry.locked);
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .locked_for_path(Path::new("keep3_direct.wav"))
            .unwrap(),
        Some(true)
    );

    controller.tag_selected(crate::sample_sources::Rating::NEUTRAL);

    let entry = controller
        .wav_entry(0)
        .expect("locked sample should stay loaded");
    assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_3);
    assert!(entry.locked);
}
