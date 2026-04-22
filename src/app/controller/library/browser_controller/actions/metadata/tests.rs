use super::*;
use crate::app::controller::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::sample_sources::db::DB_FILE_NAME;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

#[test]
fn auto_rename_request_preflight_stays_prompt_under_source_db_write_contention() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("kick.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "kick.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);
    let started_at = Instant::now();
    let requests = BrowserController::new(&mut controller)
        .prepare_auto_rename_requests(&source, &[PathBuf::from("kick.wav")])
        .expect("preflight should succeed while writer holds BEGIN IMMEDIATE");
    let elapsed = started_at.elapsed();
    release_db_lock(lock_release_tx, lock_done_rx);

    assert!(
        elapsed < Duration::from_secs(1),
        "auto-rename controller preflight waited {elapsed:?} under DB contention"
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].new_relative,
        PathBuf::from("artistname_SS_kick.wav")
    );
    assert_eq!(
        requests[0].sound_type,
        Some(crate::sample_sources::SampleSoundType::Kick)
    );
}

#[test]
fn prepare_auto_rename_requests_prefers_live_sidebar_metadata() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);

    let mut entry = sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL);
    entry.sound_type = Some(crate::sample_sources::SampleSoundType::Hat);
    entry.user_tag = Some(String::from("Live Tag"));
    controller.set_wav_entries_for_tests(vec![entry]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let db = controller.database_for(&source).unwrap();
    db.set_sound_type(
        Path::new("raw.wav"),
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    db.set_user_tag(Path::new("raw.wav"), Some("DB Tag"))
        .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("raw.wav"), Some(128.0));

    let request = BrowserController::new(&mut controller)
        .prepare_auto_rename_requests(&source, &[PathBuf::from("raw.wav")])
        .expect("request preparation should succeed")
        .into_iter()
        .next()
        .expect("request should exist");

    assert_eq!(
        request.sound_type,
        Some(crate::sample_sources::SampleSoundType::Hat)
    );
    assert_eq!(
        request.new_relative,
        PathBuf::from("artistname_SS_hat_livetag_128.wav")
    );
}

#[test]
fn resolve_auto_rename_target_skips_existing_and_reserved_names() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    write_test_wav(&source.root.join("artistname_SS_kick.wav"), &[0.0]);
    write_test_wav(&source.root.join("artistname_SS_kick_001.wav"), &[0.0]);

    let browser = BrowserController::new(&mut controller);
    let mut reserved_targets = HashSet::from([PathBuf::from("artistname_SS_kick_002.wav")]);
    let resolved = browser
        .resolve_auto_rename_target(
            &source.root,
            Path::new("raw.wav"),
            Some("artistname_SS_kick"),
            "artistname",
            &mut reserved_targets,
        )
        .expect("target resolution should succeed");

    assert_eq!(resolved, PathBuf::from("artistname_SS_kick_003.wav"));
    assert!(reserved_targets.contains(&resolved));
}

fn lock_db_until_released(source_root: &Path) -> (Sender<()>, Receiver<()>) {
    let (lock_release_tx, lock_release_rx) = std::sync::mpsc::channel();
    let (lock_done_tx, lock_done_rx) = std::sync::mpsc::channel();
    let (locked_tx, locked_rx) = std::sync::mpsc::channel();
    let db_file = source_root.join(DB_FILE_NAME);
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

fn release_db_lock(lock_release_tx: Sender<()>, lock_done_rx: Receiver<()>) {
    let _ = lock_release_tx.send(());
    lock_done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("wait for sqlite lock release");
}
