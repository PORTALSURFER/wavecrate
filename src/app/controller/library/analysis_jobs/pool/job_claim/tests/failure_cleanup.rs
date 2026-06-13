use super::*;

#[test]
fn clears_inflight_when_db_open_fails() {
    let file = NamedTempFile::new().unwrap();
    let source_root = file.path().to_path_buf();
    let job = analysis_db::ClaimedJob {
        id: 42,
        sample_id: "source::missing.wav".to_string(),
        content_hash: None,
        job_type: analysis_db::ANALYZE_SAMPLE_JOB_TYPE.to_string(),
        source_root: source_root.clone(),
    };
    let queue = DecodedQueue::new(4);
    assert!(queue.try_mark_inflight(job.id));
    let (tx, _rx) = mpsc::sync_channel::<JobMessage>(1);
    let tx = JobMessageSender::new(tx);
    let mut connections = HashMap::new();
    let progress_cache = Arc::new(RwLock::new(ProgressCache::default()));
    let progress_wakeup = ProgressPollerWakeup::new();
    let heartbeat_tracker = Arc::new(heartbeat::DecodeHeartbeatTracker::new(
        Duration::from_millis(10),
    ));
    let deferred = {
        let mut finalize = db::FinalizeJobContext {
            connections: &mut connections,
            decode_queue: &queue,
            tx: &tx,
            progress_cache: &progress_cache,
            progress_wakeup: &progress_wakeup,
            heartbeat_tracker: &heartbeat_tracker,
            log_jobs: false,
        };
        db::finalize_immediate_job(&mut finalize, job, Err("failed".to_string()))
    };

    assert!(deferred.is_some());
    assert!(queue.try_mark_inflight(42));
    assert!(connections.is_empty());
}

#[test]
fn mid_loop_db_open_failure_clears_inflight_and_marks_failed() {
    let dir = TempDir::new().unwrap();
    let source_root = dir.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let conn = analysis_db::open_source_db(&source_root).unwrap();
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at, running_at)
         VALUES (?1, ?2, ?3, ?4, 'running', 1, 0, 0)",
        rusqlite::params![
            "source::a.wav",
            "source",
            "a.wav",
            analysis_db::ANALYZE_SAMPLE_JOB_TYPE
        ],
    )
    .unwrap();
    let job_id: i64 = conn
        .query_row(
            "SELECT id FROM analysis_jobs WHERE sample_id = ?1",
            rusqlite::params!["source::a.wav"],
            |row| row.get(0),
        )
        .unwrap();
    drop(conn);
    let job = analysis_db::ClaimedJob {
        id: job_id,
        sample_id: "source::a.wav".to_string(),
        content_hash: None,
        job_type: analysis_db::ANALYZE_SAMPLE_JOB_TYPE.to_string(),
        source_root: source_root.clone(),
    };
    let queue = DecodedQueue::new(4);
    assert!(queue.try_mark_inflight(job.id));
    let (tx, _rx) = mpsc::sync_channel::<JobMessage>(1);
    let tx = JobMessageSender::new(tx);
    let progress_cache = Arc::new(RwLock::new(ProgressCache::default()));
    let progress_wakeup = ProgressPollerWakeup::new();
    let heartbeat_tracker = Arc::new(heartbeat::DecodeHeartbeatTracker::new(
        Duration::from_millis(10),
    ));
    let mut connections = HashMap::new();

    let backup_root = dir.path().join("source_backup");
    std::fs::rename(&source_root, &backup_root).unwrap();
    let deferred = {
        let mut finalize = db::FinalizeJobContext {
            connections: &mut connections,
            decode_queue: &queue,
            tx: &tx,
            progress_cache: &progress_cache,
            progress_wakeup: &progress_wakeup,
            heartbeat_tracker: &heartbeat_tracker,
            log_jobs: false,
        };
        db::finalize_immediate_job(
            &mut finalize,
            job.clone(),
            Err("Failed to open source DB".to_string()),
        )
    };
    assert!(queue.try_mark_inflight(job.id));
    assert!(deferred.is_some());

    std::fs::rename(&backup_root, &source_root).unwrap();
    let mut deferred_updates = vec![deferred.unwrap()];
    let mut finalize = db::FinalizeJobContext {
        connections: &mut connections,
        decode_queue: &queue,
        tx: &tx,
        progress_cache: &progress_cache,
        progress_wakeup: &progress_wakeup,
        heartbeat_tracker: &heartbeat_tracker,
        log_jobs: false,
    };
    db::flush_deferred_updates(&mut finalize, &mut deferred_updates);
    assert!(deferred_updates.is_empty());
    let conn = analysis_db::open_source_db(&source_root).unwrap();
    let (status, last_error): (String, Option<String>) = conn
        .query_row(
            "SELECT status, last_error FROM analysis_jobs WHERE id = ?1",
            rusqlite::params![job_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed");
    assert_eq!(last_error.as_deref(), Some("Failed to open source DB"));
}
