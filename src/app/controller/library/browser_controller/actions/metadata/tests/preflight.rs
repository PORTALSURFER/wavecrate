use super::*;
use crate::sample_sources::db::DB_FILE_NAME;
use std::sync::mpsc::{Receiver, Sender};

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
