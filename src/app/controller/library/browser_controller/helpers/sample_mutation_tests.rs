use super::sample_mutation::perform_sample_rename;
use super::sample_mutation::{
    SAMPLE_RENAME_DB_RETRIES_PRODUCTION, SAMPLE_RENAME_DB_RETRY_DELAY_PRODUCTION,
};
use super::{SampleAutoRenameRequest, run_sample_auto_rename_job};
use crate::app::controller::test_support::write_test_wav;
use crate::sample_sources::db::DB_FILE_NAME;
use crate::sample_sources::{Rating, SampleSoundType, SampleSource, SourceDatabase};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::AtomicBool,
    mpsc::{Receiver, Sender},
};
use std::time::Duration;
use tempfile::{TempDir, tempdir};

#[test]
/// Single-sample rename restores the old path when the source DB is locked.
fn sample_rename_rolls_back_file_when_db_write_cannot_start() {
    let (_temp, source) = setup_fixture(&["old.wav"]);
    let old_relative = Path::new("old.wav");
    let new_relative = Path::new("renamed.wav");
    let old_absolute = source.root.join(old_relative);
    let new_absolute = source.root.join(new_relative);
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);

    let result = perform_sample_rename(
        &source,
        &old_absolute,
        old_relative,
        new_relative,
        Rating::KEEP_3,
        false,
        false,
        None,
        Some(SampleSoundType::Kick),
        Some(String::from("Vintage")),
    );

    release_db_lock(lock_release_tx, lock_done_rx);

    let err = result.expect_err("locked DB should fail rename");
    assert!(err.contains("Failed to start database update"));
    assert!(old_absolute.is_file());
    assert!(!new_absolute.exists());

    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert_eq!(
        db.tag_for_path(old_relative).expect("old tag"),
        Some(Rating::KEEP_3)
    );
    assert_eq!(
        db.looped_for_path(old_relative).expect("old looped"),
        Some(true)
    );
    assert_eq!(
        db.locked_for_path(old_relative).expect("old locked"),
        Some(true)
    );
    assert_eq!(
        db.sound_type_for_path(old_relative)
            .expect("old sound type"),
        Some(SampleSoundType::Kick)
    );
    assert_eq!(
        db.user_tag_for_path(old_relative).expect("old user tag"),
        Some(String::from("Vintage"))
    );
    assert_eq!(
        db.last_played_at_for_path(old_relative)
            .expect("old playback age"),
        Some(42)
    );
    assert!(db.tag_for_path(new_relative).expect("new tag").is_none());
}

#[test]
/// Successful sample rename keeps the locked flag and other metadata on the new DB row.
fn sample_rename_preserves_locked_and_metadata_on_success() {
    let (_temp, source) = setup_fixture(&["old.wav"]);
    let old_relative = Path::new("old.wav");
    let new_relative = Path::new("renamed.wav");
    let old_absolute = source.root.join(old_relative);
    let new_absolute = source.root.join(new_relative);

    let entry = perform_sample_rename(
        &source,
        &old_absolute,
        old_relative,
        new_relative,
        Rating::KEEP_3,
        false,
        false,
        None,
        Some(SampleSoundType::Kick),
        Some(String::from("Vintage")),
    )
    .expect("rename should succeed");

    assert_eq!(entry.relative_path, PathBuf::from("renamed.wav"));
    assert!(entry.looped);
    assert!(entry.locked);
    assert_eq!(entry.sound_type, Some(SampleSoundType::Kick));
    assert_eq!(entry.user_tag.as_deref(), Some("Vintage"));
    assert_eq!(entry.last_played_at, Some(42));
    assert!(!old_absolute.exists());
    assert!(new_absolute.is_file());

    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert!(db.tag_for_path(old_relative).expect("old tag").is_none());
    assert_eq!(
        db.tag_for_path(new_relative).expect("new tag"),
        Some(Rating::KEEP_3)
    );
    assert_eq!(
        db.looped_for_path(new_relative).expect("new looped"),
        Some(true)
    );
    assert_eq!(
        db.locked_for_path(new_relative).expect("new locked"),
        Some(true)
    );
    assert_eq!(
        db.sound_type_for_path(new_relative)
            .expect("new sound type"),
        Some(SampleSoundType::Kick)
    );
    assert_eq!(
        db.user_tag_for_path(new_relative).expect("new user tag"),
        Some(String::from("Vintage"))
    );
    assert_eq!(
        db.last_played_at_for_path(new_relative)
            .expect("new playback age"),
        Some(42)
    );
}

#[test]
/// Auto-rename leaves every file at its original path when each DB rewrite attempt hits a busy lock.
fn sample_auto_rename_rolls_back_each_failed_file_when_db_is_busy() {
    let (_temp, source) = setup_fixture(&["alpha.wav", "beta.wav"]);
    let requests = vec![
        rename_request("alpha.wav", "alpha_renamed.wav"),
        rename_request("beta.wav", "beta_renamed.wav"),
    ];
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);

    let result =
        run_sample_auto_rename_job(source.clone(), requests, Arc::new(AtomicBool::new(false)));

    release_db_lock(lock_release_tx, lock_done_rx);

    assert!(result.renamed.is_empty());
    assert!(result.skipped.is_empty());
    assert_eq!(result.errors.len(), 2);
    assert!(
        result
            .errors
            .iter()
            .all(|(_, err)| err.contains("Failed to start database update"))
    );
    assert!(source.root.join("alpha.wav").is_file());
    assert!(source.root.join("beta.wav").is_file());
    assert!(!source.root.join("alpha_renamed.wav").exists());
    assert!(!source.root.join("beta_renamed.wav").exists());

    let db = SourceDatabase::open(&source.root).expect("open source db");
    for relative in [Path::new("alpha.wav"), Path::new("beta.wav")] {
        assert_eq!(
            db.tag_for_path(relative).expect("tag"),
            Some(Rating::KEEP_3)
        );
        assert_eq!(db.locked_for_path(relative).expect("locked"), Some(true));
        assert_eq!(
            db.user_tag_for_path(relative).expect("user tag"),
            Some(String::from("Vintage"))
        );
    }
}

#[test]
fn production_sample_rename_retry_budget_covers_multi_second_busy_windows() {
    assert!(
        SAMPLE_RENAME_DB_RETRY_DELAY_PRODUCTION
            .saturating_mul(SAMPLE_RENAME_DB_RETRIES_PRODUCTION as u32)
            >= Duration::from_millis(5_500)
    );
}

#[test]
/// Auto-rename waits past the old 200 ms retry budget instead of rolling back the file rename.
fn sample_auto_rename_retries_until_multi_attempt_db_lock_clears() {
    let (_temp, source) = setup_fixture(&["alpha.wav"]);
    let requests = vec![rename_request("alpha.wav", "alpha_renamed.wav")];
    let (lock_release_tx, lock_done_rx) = lock_db_until_released(&source.root);

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(260));
        release_db_lock(lock_release_tx, lock_done_rx);
    });

    let result =
        run_sample_auto_rename_job(source.clone(), requests, Arc::new(AtomicBool::new(false)));

    assert!(
        result.errors.is_empty(),
        "rename should retry through short lock"
    );
    assert!(result.skipped.is_empty());
    assert_eq!(result.renamed.len(), 1);
    assert!(!source.root.join("alpha.wav").exists());
    assert!(source.root.join("alpha_renamed.wav").is_file());

    let db = SourceDatabase::open(&source.root).expect("open source db");
    assert!(
        db.tag_for_path(Path::new("alpha.wav"))
            .expect("old tag")
            .is_none()
    );
    assert_eq!(
        db.tag_for_path(Path::new("alpha_renamed.wav"))
            .expect("renamed tag"),
        Some(Rating::KEEP_3)
    );
    assert_eq!(
        db.locked_for_path(Path::new("alpha_renamed.wav"))
            .expect("renamed locked"),
        Some(true)
    );
}

#[test]
/// Auto-rename persists inferred sound type in the worker when the old DB row is missing it.
fn sample_auto_rename_persists_inferred_sound_type_without_controller_db_write() {
    let temp = tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let relative = Path::new("mystery.wav");
    let absolute = source.root.join(relative);
    write_test_wav(&absolute, &[0.0, 0.1, -0.1]);
    let metadata = std::fs::metadata(&absolute).expect("read file metadata");
    let db = SourceDatabase::open(&source.root).expect("open source db");
    db.upsert_file(relative, metadata.len(), 0)
        .expect("insert db row");
    db.set_tag(relative, Rating::KEEP_3).expect("set tag");

    let result = run_sample_auto_rename_job(
        source.clone(),
        vec![SampleAutoRenameRequest {
            old_relative: relative.to_path_buf(),
            new_relative: PathBuf::from("portal_SS_kick.wav"),
            tag: Rating::KEEP_3,
            looped: false,
            locked: false,
            sound_type: Some(SampleSoundType::Kick),
            user_tag: None,
            last_played_at: None,
            resume_playback: false,
            resume_looped: false,
            resume_start_override: None,
        }],
        Arc::new(AtomicBool::new(false)),
    );

    assert!(result.errors.is_empty());
    assert_eq!(result.renamed.len(), 1);
    let renamed = Path::new("portal_SS_kick.wav");
    let db = SourceDatabase::open(&source.root).expect("reopen source db");
    assert_eq!(
        db.sound_type_for_path(renamed).expect("renamed sound type"),
        Some(SampleSoundType::Kick)
    );
}

#[test]
fn repeated_sample_auto_rename_preserves_analysis_artifacts() {
    let (_temp, source) = setup_fixture(&["alpha.wav"]);
    let first = run_sample_auto_rename_job(
        source.clone(),
        vec![rename_request("alpha.wav", "alpha_renamed.wav")],
        Arc::new(AtomicBool::new(false)),
    );
    assert!(first.errors.is_empty());

    let second = run_sample_auto_rename_job(
        source.clone(),
        vec![rename_request("alpha_renamed.wav", "alpha_final.wav")],
        Arc::new(AtomicBool::new(false)),
    );
    assert!(second.errors.is_empty());

    let conn = rusqlite::Connection::open(source.root.join(DB_FILE_NAME)).expect("open sqlite");
    let old_sample_id = format!("{}::alpha.wav", source.id);
    let first_sample_id = format!("{}::alpha_renamed.wav", source.id);
    let final_sample_id = format!("{}::alpha_final.wav", source.id);
    for table in ["samples", "features", "embeddings", "analysis_jobs"] {
        assert_eq!(
            sample_id_count(&conn, table, &old_sample_id),
            0,
            "{table} retained old identity"
        );
        assert_eq!(
            sample_id_count(&conn, table, &first_sample_id),
            0,
            "{table} retained intermediate identity"
        );
        assert_eq!(
            sample_id_count(&conn, table, &final_sample_id),
            1,
            "{table} did not remap to final identity"
        );
    }
    let pending_jobs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pending_jobs, 0);
    let job_relative: String = conn
        .query_row(
            "SELECT relative_path FROM analysis_jobs WHERE sample_id = ?1",
            [&final_sample_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(job_relative, "alpha_final.wav");
}

fn setup_fixture(names: &[&str]) -> (TempDir, SampleSource) {
    let temp = tempdir().expect("create temp dir");
    let source = SampleSource::new(temp.path().join("source"));
    std::fs::create_dir_all(&source.root).expect("create source root");
    let db = SourceDatabase::open(&source.root).expect("open source db");
    for name in names {
        let relative = Path::new(name);
        let absolute = source.root.join(relative);
        write_test_wav(&absolute, &[0.0, 0.1, -0.1]);
        let metadata = std::fs::metadata(&absolute).expect("read file metadata");
        db.upsert_file(relative, metadata.len(), 0)
            .expect("insert db row");
        db.set_tag(relative, Rating::KEEP_3).expect("set tag");
        db.set_looped(relative, true).expect("set looped");
        db.set_locked(relative, true).expect("set locked");
        db.set_sound_type(relative, Some(SampleSoundType::Kick))
            .expect("set sound type");
        db.set_user_tag(relative, Some("Vintage"))
            .expect("set user tag");
        db.set_last_played_at(relative, 42)
            .expect("set playback age");
        insert_analysis_artifacts(&source, relative);
    }
    (temp, source)
}

fn insert_analysis_artifacts(source: &SampleSource, relative: &Path) {
    let conn = rusqlite::Connection::open(source.root.join(DB_FILE_NAME)).expect("open sqlite");
    let sample_id = format!(
        "{}::{}",
        source.id,
        relative.to_string_lossy().replace('\\', "/")
    );
    conn.execute(
        "INSERT INTO samples (
             sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version
         ) VALUES (?1, 'hash-a', 1, 1, 1.0, 48000, 'analysis_v1_test')",
        [&sample_id],
    )
    .expect("insert sample analysis row");
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
         VALUES (?1, 1, x'00', x'00', 0.0, 1)",
        [&sample_id],
    )
    .expect("insert features");
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, 'model', 1, 'f32', 1, x'00', 1)",
        [&sample_id],
    )
    .expect("insert embeddings");
    conn.execute(
        "INSERT INTO analysis_jobs (
             sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at
         ) VALUES (?1, ?2, ?3, 'analyze_sample', 'hash-a', 'done', 0, 1)",
        rusqlite::params![sample_id, source.id.as_str(), relative.to_string_lossy()],
    )
    .expect("insert analysis job");
}

fn sample_id_count(conn: &rusqlite::Connection, table: &str, sample_id: &str) -> i64 {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE sample_id = ?1"),
        [sample_id],
        |row| row.get(0),
    )
    .unwrap()
}

fn rename_request(old_relative: &str, new_relative: &str) -> SampleAutoRenameRequest {
    SampleAutoRenameRequest {
        old_relative: PathBuf::from(old_relative),
        new_relative: PathBuf::from(new_relative),
        tag: Rating::KEEP_3,
        looped: true,
        locked: true,
        sound_type: Some(SampleSoundType::Kick),
        user_tag: Some(String::from("Vintage")),
        last_played_at: Some(42),
        resume_playback: false,
        resume_looped: false,
        resume_start_override: None,
    }
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
