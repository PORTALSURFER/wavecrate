use super::*;

#[test]
fn stale_loop_failure_does_not_undo_newer_one_shot_selection() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    let mut entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.looped = true;
    register_entry_metadata(&mut controller, &source, &entry);
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.tag_sidebar_open = true;
    controller.focus_browser_row_only(0);

    let stale_request_id = 5252;
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
        .apply_browser_tag_sidebar_looped(false)
        .expect("newer one-shot click should apply");
    assert_sidebar_loop_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::Off,
    );
    assert_sidebar_one_shot_state(
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

    assert!(!controller.wav_entry(0).unwrap().looped);
    assert_sidebar_loop_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::Off,
    );
    assert_sidebar_one_shot_state(
        &mut controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );
}
