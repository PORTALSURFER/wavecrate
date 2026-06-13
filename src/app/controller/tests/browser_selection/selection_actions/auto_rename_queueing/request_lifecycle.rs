use super::*;

#[test]
fn repeated_auto_rename_for_active_target_collapses_without_warning_churn() {
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
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(PathBuf::from("raw.wav"), PathBuf::from("portal_SS.wav"))],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    controller.auto_rename_browser_selection_action(Some(0));
    controller.auto_rename_browser_selection_action(Some(0));

    assert_eq!(
        controller.ui.status.text,
        "Auto rename already in progress..."
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Busy
    );
    assert!(source.root.join("raw.wav").exists());
    assert!(!source.root.join("portal_SS.wav").exists());
}

#[test]
fn different_auto_rename_request_queues_one_follow_up_after_active_rename_finishes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["raw.wav", "kick.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    let mut kick = sample_entry("kick.wav", crate::sample_sources::Rating::NEUTRAL);
    kick.sound_type = Some(crate::sample_sources::SampleSoundType::Kick);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL),
        kick,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(PathBuf::from("raw.wav"), PathBuf::from("portal_SS.wav"))],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    controller.focus_browser_row_only(1);
    controller.auto_rename_browser_selection_action(Some(1));
    assert_eq!(
        controller.ui.status.text,
        "Auto rename queued after current rename..."
    );

    controller.runtime.jobs.clear_file_ops();
    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id: source.id.clone(),
                requested_paths: vec![PathBuf::from("raw.wav")],
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: vec![(PathBuf::from("raw.wav"), String::from("Rename cancelled"))],
            },
        ),
    );

    assert!(source.root.join("raw.wav").exists());
    assert!(!source.root.join("kick.wav").exists());
    assert!(source.root.join("portal_SS_kick.wav").exists());
}

#[test]
fn queued_auto_rename_replays_against_active_rename_success_path() {
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
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(PathBuf::from("raw.wav"), PathBuf::from("portal_SS.wav"))],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );

    controller
        .apply_browser_tag_sidebar_sound_type(Some(crate::sample_sources::SampleSoundType::Kick))
        .expect("sound type should apply");
    controller.auto_rename_browser_selection_action(Some(0));

    let first_relative = Path::new("portal_SS.wav");
    std::fs::rename(
        source.root.join("raw.wav"),
        source.root.join(first_relative),
    )
    .unwrap();
    let db = controller.database_for(&source).unwrap();
    db.remove_file(Path::new("raw.wav")).unwrap();
    db.upsert_file(first_relative, 0, 0).unwrap();
    db.set_tag(first_relative, crate::sample_sources::Rating::NEUTRAL)
        .unwrap();
    db.set_sound_type(
        first_relative,
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    let mut entry = sample_entry("portal_SS.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.sound_type = Some(crate::sample_sources::SampleSoundType::Kick);

    controller.runtime.jobs.clear_file_ops();
    controller.apply_file_op_result(
        crate::app::controller::jobs::FileOpResult::SampleAutoRename(
            crate::app::controller::jobs::SampleAutoRenameResult {
                source_id: source.id.clone(),
                requested_paths: vec![PathBuf::from("raw.wav")],
                renamed: vec![crate::app::controller::jobs::SampleAutoRenameSuccess {
                    old_relative: PathBuf::from("raw.wav"),
                    new_relative: PathBuf::from("portal_SS.wav"),
                    entry,
                    resume_playback: false,
                    resume_looped: false,
                    resume_start_override: None,
                }],
                skipped: Vec::new(),
                errors: Vec::new(),
            },
        ),
    );

    assert!(!source.root.join("raw.wav").exists());
    assert!(!source.root.join("portal_SS.wav").exists());
    assert!(source.root.join("portal_SS_kick.wav").exists());
}
