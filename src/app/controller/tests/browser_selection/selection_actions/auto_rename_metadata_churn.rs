use super::*;
use std::io;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    fn captured(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedBufferWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedBufferWriter(self.0.clone())
    }
}

struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for SharedBufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn capture_info_logs<F>(run: F) -> String
where
    F: FnOnce(),
{
    let buffer = SharedBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .with_writer(buffer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, run);
    buffer.captured()
}

#[test]
fn tag_sidebar_auto_rename_logs_metadata_provenance() {
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

    let captured = capture_info_logs(|| {
        controller
            .apply_browser_tag_sidebar_looped(true)
            .expect("loop click should auto rename");
    });

    assert!(
        captured.contains("auto rename: request metadata provenance")
            && captured.contains("raw.wav -> portal_loop.wav looped=true"),
        "tag-sidebar auto-rename should log requested loop provenance: {captured}"
    );
    assert!(
        captured.contains("auto rename: persisted loop metadata provenance")
            && captured.contains("old_path=raw.wav")
            && captured.contains("new_path=portal_loop.wav")
            && captured.contains("request_looped=true")
            && captured.contains("db_looped=Some(true)")
            && captured.contains("final_looped=true"),
        "tag-sidebar auto-rename should log DB and final loop provenance: {captured}"
    );
    assert!(
        captured.contains("source metadata mutation: source ops resolved")
            && captured.contains("SetLooped raw.wav")
            && captured.contains("result=\"ok\""),
        "tag-sidebar metadata write should log operation names and result: {captured}"
    );
}

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

#[test]
fn multi_step_auto_rename_stays_stable_under_metadata_and_maintenance_churn() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();

    let mut kick = sample_entry("raw_kick.wav", crate::sample_sources::Rating::NEUTRAL);
    kick.sound_type = Some(crate::sample_sources::SampleSoundType::Kick);
    let mut snare = sample_entry("raw_snare.wav", crate::sample_sources::Rating::NEUTRAL);
    snare.sound_type = Some(crate::sample_sources::SampleSoundType::Snare);
    let hat = sample_entry("raw_hat.wav", crate::sample_sources::Rating::NEUTRAL);
    let entries = vec![kick, snare, hat];
    for entry in &entries {
        write_test_wav(&source.root.join(&entry.relative_path), &[0.0]);
        register_entry_metadata(&mut controller, &source, entry);
    }
    controller.set_wav_entries_for_tests(entries);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.auto_rename_browser_selection_action(Some(0));

    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 2, skipped 0, failed 0"
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Info
    );
    assert!(source.root.join("artistname_SS_kick.wav").exists());
    assert!(source.root.join("artistname_SS_snare.wav").exists());
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![
            PathBuf::from("artistname_SS_kick.wav"),
            PathBuf::from("artistname_SS_snare.wav")
        ]
    );
    assert_eq!(controller.wav_entries.total, 3);

    controller.set_browser_selected_paths(Vec::new());
    controller.focus_browser_row_only(2);
    controller
        .apply_browser_tag_sidebar_user_tag(Some(String::from("Vintage FX")))
        .expect("custom tag should apply");
    controller
        .apply_browser_tag_sidebar_sound_type(Some(crate::sample_sources::SampleSoundType::Hat))
        .expect("sound type should apply");
    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("loop tag should apply");
    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: 99,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw_hat.wav")].into_iter().collect(),
                blocks_file_mutation: false,
                rollback: Vec::new(),
                refresh_browser_projection: false,
            },
        );

    controller
        .runtime
        .source_lane
        .mutations
        .begin_browser_rename_intent(
            crate::app::controller::state::runtime::BrowserRenameIntentKey::new(
                source.id.clone(),
                vec![(
                    PathBuf::from("raw_hat.wav"),
                    PathBuf::from("artistname_loop_hat_vintagefx.wav"),
                )],
            ),
        );
    let (_file_op_tx, file_op_rx) =
        std::sync::mpsc::channel::<crate::app::controller::jobs::FileOpMessage>();
    controller.runtime.jobs.start_file_ops(
        file_op_rx,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );
    controller
        .runtime
        .deferred_startup_source_db_maintenance_jobs =
        vec![crate::app::controller::jobs::SourceDbMaintenanceJob {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
        }];
    controller
        .runtime
        .deferred_startup_source_db_maintenance_armed = true;
    controller.runtime.startup_frame_prepare_count = 1;
    controller.begin_pending_file_mutation(&source.id, [PathBuf::from("raw_hat.wav")]);
    controller.flush_deferred_startup_source_db_maintenance();

    assert!(controller.has_pending_startup_source_db_maintenance());
    controller.auto_rename_browser_selection_action(Some(2));
    controller.auto_rename_browser_selection_action(Some(2));
    assert_eq!(
        controller.ui.status.text,
        "Auto rename already in progress..."
    );
    assert!(source.root.join("raw_hat.wav").exists());
    assert!(
        !source
            .root
            .join("artistname_loop_hat_vintagefx.wav")
            .exists()
    );

    controller.runtime.jobs.clear_file_ops();
    let _ = controller
        .runtime
        .source_lane
        .mutations
        .finish_browser_rename_intent();
    controller.finish_pending_file_mutation(&source.id, [PathBuf::from("raw_hat.wav")]);
    controller.flush_deferred_startup_source_db_maintenance();
    assert!(!controller.has_pending_startup_source_db_maintenance());
    controller.set_browser_selected_paths(Vec::new());

    let (lock_release_tx, lock_done_rx) = lock_source_db_until_released(&source.root);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        release_source_db_lock(lock_release_tx, lock_done_rx);
    });
    controller.auto_rename_browser_selection_action(Some(2));

    let final_hat = PathBuf::from("artistname_loop_hat_vintagefx.wav");
    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 1, skipped 0, failed 0"
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Info
    );
    assert!(!source.root.join("raw_hat.wav").exists());
    assert!(source.root.join(&final_hat).exists());
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(final_hat.as_path())
    );
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(controller.wav_entries.total, 3);

    controller.apply_background_job_message_for_tests(
        crate::app::controller::jobs::JobMessage::SourceDbMaintenanceFinished(
            crate::app::controller::jobs::SourceDbMaintenanceResult {
                outcomes: vec![crate::app::controller::jobs::SourceDbMaintenanceOutcome {
                    source_id: source.id.clone(),
                    source_root: source.root.clone(),
                    skipped: false,
                    deferred_due_to_file_op: true,
                    orphan_rows_removed: 0,
                    refresh:
                        crate::app::controller::jobs::SourceDbMaintenanceRefresh::FileOpReconcile,
                    error: None,
                }],
            },
        ),
    );
    assert_eq!(controller.wav_entries.total, 3);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(final_hat.as_path())
    );

    controller.apply_background_job_message_for_tests(
        crate::app::controller::jobs::JobMessage::MetadataMutationFinished(
            crate::app::controller::jobs::MetadataMutationResult {
                request_id: 99,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw_hat.wav")].into_iter().collect(),
                elapsed: std::time::Duration::from_millis(5),
                result: Err(String::from(
                    "Failed to start analysis metadata transaction: database is locked",
                )),
            },
        ),
    );
    assert_eq!(
        controller.ui.status.text,
        "Auto Rename: renamed 1, skipped 0, failed 0"
    );
    assert_eq!(
        controller.ui.status.status_tone,
        crate::app::state::StatusTone::Info
    );
}

fn assert_sidebar_loop_state(
    controller: &mut AppController,
    expected: crate::app_core::actions::NativeBrowserTagState,
) {
    let model = crate::app_core::native_shell::project_browser_tag_sidebar_model(controller);
    assert_eq!(model.playback_type_pills[0].state, expected);
}

fn assert_sidebar_one_shot_state(
    controller: &mut AppController,
    expected: crate::app_core::actions::NativeBrowserTagState,
) {
    let model = crate::app_core::native_shell::project_browser_tag_sidebar_model(controller);
    assert_eq!(model.playback_type_pills[1].state, expected);
}

fn assert_renamed_loop_surfaces(
    controller: &mut AppController,
    source: &SampleSource,
    renamed: &Path,
) {
    let entry_index = controller
        .wav_index_for_path(renamed)
        .expect("renamed entry should stay cached");
    assert!(
        controller.wav_entry(entry_index).unwrap().looped,
        "cached WavEntry should remain Loop after auto-rename"
    );
    let projected = crate::app_core::native_shell::project_browser_model(controller);
    let row = projected
        .rows
        .iter()
        .find(|row| row.label.as_ref() == "portal_loop")
        .expect("renamed row should be visible in the browser projection");
    assert_eq!(
        row.bucket_label.as_deref(),
        Some("LOOP"),
        "visible browser row should still project Loop"
    );
    assert_sidebar_loop_state(
        controller,
        crate::app_core::actions::NativeBrowserTagState::On,
    );
    assert_sidebar_one_shot_state(
        controller,
        crate::app_core::actions::NativeBrowserTagState::Off,
    );
    assert_eq!(
        controller
            .database_for(source)
            .unwrap()
            .looped_for_path(renamed)
            .unwrap(),
        Some(true),
        "source DB row for the renamed path should persist Loop"
    );
}

fn register_entry_metadata(
    controller: &mut AppController,
    source: &SampleSource,
    entry: &crate::sample_sources::WavEntry,
) {
    let metadata = std::fs::metadata(source.root.join(&entry.relative_path)).unwrap();
    let db = controller.database_for(source).unwrap();
    db.upsert_file(&entry.relative_path, metadata.len(), 0)
        .unwrap();
    db.set_tag(&entry.relative_path, entry.tag).unwrap();
    db.set_looped(&entry.relative_path, entry.looped).unwrap();
    db.set_locked(&entry.relative_path, entry.locked).unwrap();
    db.set_sound_type(&entry.relative_path, entry.sound_type)
        .unwrap();
    db.set_user_tag(&entry.relative_path, entry.user_tag.as_deref())
        .unwrap();
}

fn lock_source_db_until_released(
    source_root: &Path,
) -> (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(crate::sample_sources::db::DB_FILE_NAME);
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open(db_file).expect("open sqlite lock connection");
        conn.execute_batch("BEGIN IMMEDIATE")
            .expect("start immediate transaction");
        let _ = locked_tx.send(());
        let _ = lock_release_rx.recv();
        let _ = conn.execute_batch("COMMIT");
        let _ = lock_done_tx.send(());
    });
    locked_rx.recv().expect("wait for sqlite lock");
    (lock_release_tx, lock_done_rx)
}

fn release_source_db_lock(
    lock_release_tx: std::sync::mpsc::Sender<()>,
    lock_done_rx: std::sync::mpsc::Receiver<()>,
) {
    let _ = lock_release_tx.send(());
    lock_done_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("wait for sqlite lock release");
}
