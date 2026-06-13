use super::*;

#[test]
fn shared_decode_heartbeat_keeps_running_job_fresh() {
    let dir = TempDir::new().unwrap();
    let conn = analysis_db::open_source_db(dir.path()).unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64;
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, job_type, status, attempts, created_at, running_at)
         VALUES (?1, ?2, 'running', 1, ?3, ?4)",
        rusqlite::params![
            "source::long.wav",
            analysis_db::ANALYZE_SAMPLE_JOB_TYPE,
            now,
            now - 120
        ],
    )
    .unwrap();
    let job_id: i64 = conn
        .query_row(
            "SELECT id FROM analysis_jobs WHERE sample_id = ?1",
            rusqlite::params!["source::long.wav"],
            |row| row.get(0),
        )
        .unwrap();

    let tracker = Arc::new(heartbeat::DecodeHeartbeatTracker::new(
        Duration::from_millis(10),
    ));
    let handle = heartbeat::spawn_decode_heartbeat_worker(tracker.clone());
    tracker.register(dir.path(), job_id);
    let deadline = Instant::now() + Duration::from_secs(2);
    let heartbeat_observed = loop {
        let running_at: Option<i64> = conn
            .query_row(
                "SELECT running_at FROM analysis_jobs WHERE id = ?1",
                rusqlite::params![job_id],
                |row| row.get(0),
            )
            .unwrap_or(None);
        if running_at.is_some_and(|ts| ts >= now - 1) {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        sleep(Duration::from_millis(10));
    };
    let stale_before = now - 1;
    let changed = analysis_db::fail_stale_running_jobs(&conn, stale_before).unwrap();
    tracker.unregister(dir.path(), job_id);
    tracker.close();
    let _ = handle.join();

    assert!(
        heartbeat_observed,
        "heartbeat should refresh running_at before stale cleanup"
    );
    assert_eq!(changed, 0);
}
