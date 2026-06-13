use super::*;

#[test]
fn repeated_loop_sidebar_click_survives_auto_rename_and_stale_metadata_failure() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    let entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    register_entry_metadata(&mut controller, &source, &entry);
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.tag_sidebar_auto_rename = true;
    controller.focus_browser_row_only(0);

    let stale_request_id = 4242;
    let stale_intent_id = controller
        .runtime
        .source_lane
        .mutations
        .begin_looped_metadata_intent(&source.id, Path::new("raw.wav"));
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: stale_request_id,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                blocks_file_mutation: true,
                rollback: vec![
                    crate::app::controller::state::runtime::MetadataRollback::Looped {
                        relative_path: PathBuf::from("raw.wav"),
                        intent_id: stale_intent_id,
                        before_looped: false,
                        expected_looped: true,
                    },
                ],
                refresh_browser_projection: true,
            },
        );

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("loop click should apply optimistically and auto rename");

    let renamed = PathBuf::from("portal_loop.wav");
    assert!(source.root.join(&renamed).exists());
    assert!(!source.root.join("raw.wav").exists());
    assert_sidebar_loop_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );
    assert_sidebar_one_shot_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::Off,
    );

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("repeated loop click should coalesce to latest intent");
    assert_sidebar_loop_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );

    controller.apply_background_job_message_for_tests(
        crate::app::controller::jobs::JobMessage::MetadataMutationFinished(
            crate::app::controller::jobs::MetadataMutationResult {
                request_id: stale_request_id,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                elapsed: std::time::Duration::from_millis(5),
                result: Err(String::from("forced stale loop metadata failure")),
            },
        ),
    );

    let renamed_index = controller
        .wav_index_for_path(&renamed)
        .expect("renamed entry should stay cached");
    assert!(controller.wav_entry(renamed_index).unwrap().looped);
    assert_sidebar_loop_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );
    assert_sidebar_one_shot_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::Off,
    );
}

#[test]
fn loop_sidebar_auto_rename_keeps_loop_when_source_db_still_has_stale_one_shot() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    let entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    register_entry_metadata(&mut controller, &source, &entry);
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.tag_sidebar_open = true;
    controller.ui.browser.tag_sidebar_auto_rename = true;
    controller.focus_browser_row_only(0);

    // Reproduce the live ordering: the sidebar click has already updated the
    // controller row, but the source DB still stores the old One-shot value.
    let stale_request_id = 6262;
    let stale_intent_id = controller
        .runtime
        .source_lane
        .mutations
        .begin_looped_metadata_intent(&source.id, Path::new("raw.wav"));
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: stale_request_id,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                blocks_file_mutation: true,
                rollback: vec![
                    crate::app::controller::state::runtime::MetadataRollback::Looped {
                        relative_path: PathBuf::from("raw.wav"),
                        intent_id: stale_intent_id,
                        before_looped: false,
                        expected_looped: true,
                    },
                ],
                refresh_browser_projection: true,
            },
        );
    let raw_index = controller
        .wav_index_for_path(Path::new("raw.wav"))
        .expect("raw entry should be cached");
    controller.wav_entries.entry_mut(raw_index).unwrap().looped = true;
    controller.mark_browser_row_metadata_projection_revision_dirty();
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .looped_for_path(Path::new("raw.wav"))
            .unwrap(),
        Some(false),
        "source DB must still expose the stale one-shot value"
    );
    assert_sidebar_loop_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );

    controller
        .browser()
        .auto_rename_browser_sample_paths_action(&[PathBuf::from("raw.wav")])
        .expect("sidebar auto-rename should run after optimistic Loop click");

    let renamed = PathBuf::from("portal_loop.wav");
    assert!(source.root.join(&renamed).exists());
    assert!(!source.root.join("raw.wav").exists());
    assert_renamed_loop_surfaces(&mut controller, &source, &renamed);

    controller.apply_background_job_message_for_tests(
        crate::app::controller::jobs::JobMessage::MetadataMutationFinished(
            crate::app::controller::jobs::MetadataMutationResult {
                request_id: stale_request_id,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                elapsed: std::time::Duration::from_millis(5),
                result: Ok(()),
            },
        ),
    );

    assert_renamed_loop_surfaces(&mut controller, &source, &renamed);
}
