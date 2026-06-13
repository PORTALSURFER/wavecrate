use super::*;
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
/// Handles metadata rollback uses mutation source after source switch.
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
/// Verifies metadata rollback restores variant active cache and ui state.
fn metadata_rollback_restores_variant_active_cache_and_ui_state() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let mut entry = sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.looped = true;
    entry.sound_type = Some(crate::sample_sources::SampleSoundType::Kick);
    entry.user_tag = Some(String::from("New Tag"));
    entry.last_played_at = Some(20);
    entry.normal_tags = vec![String::from("Vintage FX")];
    controller.set_wav_entries_for_tests(vec![entry.clone()]);
    controller
        .cache
        .wav
        .insert_page(source.id.clone(), 1, 100, 0, vec![entry]);
    controller
        .ui_cache
        .browser
        .normal_tags
        .entry(source.id.clone())
        .or_default()
        .insert(
            PathBuf::from("one.wav"),
            vec![crate::sample_sources::db::SourceTag {
                id: 0,
                display_label: String::from("Vintage FX"),
                normalized_text: String::from("vintage fx"),
            }],
        );
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("one.wav"), Some(140.0));
    let loop_intent = controller
        .runtime
        .source_lane
        .mutations
        .begin_looped_metadata_intent(&source.id, Path::new("one.wav"));

    let request_id = 8282;
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id,
                source_id: source.id.clone(),
                paths: [PathBuf::from("one.wav")].into_iter().collect(),
                blocks_file_mutation: true,
                rollback: vec![
                    crate::app::controller::state::runtime::MetadataRollback::Looped {
                        relative_path: PathBuf::from("one.wav"),
                        intent_id: loop_intent,
                        before_looped: false,
                        expected_looped: true,
                    },
                    crate::app::controller::state::runtime::MetadataRollback::SoundType {
                        relative_path: PathBuf::from("one.wav"),
                        before_sound_type: None,
                        expected_sound_type: Some(crate::sample_sources::SampleSoundType::Kick),
                    },
                    crate::app::controller::state::runtime::MetadataRollback::UserTag {
                        relative_path: PathBuf::from("one.wav"),
                        before_user_tag: Some(String::from("Old Tag")),
                        expected_user_tag: Some(String::from("New Tag")),
                    },
                    crate::app::controller::state::runtime::MetadataRollback::NormalTag {
                        relative_path: PathBuf::from("one.wav"),
                        normalized_text: String::from("vintage fx"),
                        display_label: String::from("Vintage FX"),
                        before_present: false,
                        expected_present: true,
                    },
                    crate::app::controller::state::runtime::MetadataRollback::LastPlayedAt {
                        relative_path: PathBuf::from("one.wav"),
                        before_last_played_at: Some(10),
                        expected_last_played_at: Some(20),
                    },
                    crate::app::controller::state::runtime::MetadataRollback::Bpm {
                        relative_path: PathBuf::from("one.wav"),
                        before_bpm: Some(120.0),
                        expected_bpm: Some(140.0),
                    },
                ],
                refresh_browser_projection: true,
            },
        );

    controller.handle_metadata_mutation_finished_message(
        crate::app::controller::jobs::MetadataMutationResult {
            request_id,
            source_id: source.id.clone(),
            paths: [PathBuf::from("one.wav")].into_iter().collect(),
            elapsed: std::time::Duration::ZERO,
            result: Err(String::from("forced metadata failure")),
        },
    );

    let active = controller.wav_entry(0).unwrap();
    assert!(!active.looped);
    assert_eq!(active.sound_type, None);
    assert_eq!(active.user_tag.as_deref(), Some("Old Tag"));
    assert!(active.normal_tags.is_empty());
    assert_eq!(active.last_played_at, Some(10));

    let source_cache = controller.cache.wav.entries.get(&source.id).unwrap();
    let source_index = source_cache
        .lookup
        .get(Path::new("one.wav"))
        .copied()
        .unwrap();
    let cached = source_cache.entry(source_index).unwrap();
    assert!(!cached.looped);
    assert_eq!(cached.sound_type, None);
    assert_eq!(cached.user_tag.as_deref(), Some("Old Tag"));
    assert!(cached.normal_tags.is_empty());
    assert_eq!(cached.last_played_at, Some(10));
    assert!(
        controller
            .ui_cache
            .browser
            .normal_tags
            .get(&source.id)
            .and_then(|tags| tags.get(Path::new("one.wav")))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        controller
            .ui_cache
            .browser
            .bpm_values
            .get(&source.id)
            .and_then(|values| values.get(Path::new("one.wav")))
            .copied()
            .flatten(),
        Some(120.0)
    );
}
