mod aggregate;
mod cleanup;
#[cfg(not(test))]
mod poller;
mod source_discovery;
mod wakeup;

use std::time::Duration;

const SOURCE_REFRESH_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const STALE_CLEANUP_INTERVAL: Duration = Duration::from_secs(10);

pub(crate) use wakeup::ProgressPollerWakeup;

#[cfg(not(test))]
pub(crate) use poller::spawn_progress_poller;

#[cfg(test)]
mod tests {
    use super::SOURCE_REFRESH_INTERVAL;
    use super::aggregate::{current_progress_all, seed_missing_progress};
    use super::cleanup::{cleanup_stale_jobs, now_epoch_seconds};
    use super::source_discovery::{ProgressSourceDb, refresh_sources};
    use crate::app::controller::jobs::{JobMessage, JobMessageSender};
    use crate::app::controller::library::analysis_jobs::db;
    use crate::app::controller::library::analysis_jobs::types::{
        AnalysisJobMessage, AnalysisProgress,
    };
    use crate::app::controller::library::source_write_priority::FileOpWritePriorityGuard;
    use radiant::gui::repaint::SharedRepaintSignal;
    use std::sync::{Arc, RwLock};
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    use super::super::progress_cache::ProgressCache;
    use super::wakeup::ProgressPollerWakeup;

    #[test]
    fn cleanup_runs_without_workers() {
        let dir = TempDir::new().unwrap();
        let conn = db::open_source_db_maintenance(dir.path()).unwrap();
        let now = now_epoch_seconds();
        conn.execute(
            "INSERT INTO analysis_jobs (sample_id, job_type, status, attempts, created_at, running_at)
             VALUES (?1, ?2, 'running', 1, ?3, ?4)",
            rusqlite::params![
                "source::stale.wav",
                db::ANALYZE_SAMPLE_JOB_TYPE,
                now,
                now - 120
            ],
        )
        .unwrap();
        let mut sources = vec![ProgressSourceDb {
            source_id: crate::sample_sources::SourceId::from_string("source".to_string()),
            source_root: dir.path().to_path_buf(),
            conn,
        }];
        let cache = Arc::new(RwLock::new(ProgressCache::default()));
        let (tx, _rx) = std::sync::mpsc::sync_channel(1);
        let tx = JobMessageSender::new(tx);
        let stale_before = now - 10;
        let signal = Arc::new(SharedRepaintSignal::default());

        let changed = cleanup_stale_jobs(&mut sources, stale_before, &cache, &tx, &signal);

        let status: String = sources[0]
            .conn
            .query_row(
                "SELECT status FROM analysis_jobs WHERE sample_id = ?1",
                rusqlite::params!["source::stale.wav"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(changed, 1);
        assert_eq!(status, "failed");
    }

    #[test]
    fn cleanup_updates_cache_and_emits_message() {
        let dir = TempDir::new().unwrap();
        let conn = db::open_source_db_maintenance(dir.path()).unwrap();
        conn.execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, missing)
             VALUES (?1, 1, 0, 0)",
            rusqlite::params!["a.wav"],
        )
        .unwrap();
        let now = now_epoch_seconds();
        conn.execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at, running_at)
             VALUES (?1, ?2, ?3, ?4, 'running', 1, ?5, ?6)",
            rusqlite::params![
                "source::a.wav",
                "source",
                "a.wav",
                db::ANALYZE_SAMPLE_JOB_TYPE,
                now,
                now - 120
            ],
        )
        .unwrap();
        let mut sources = vec![ProgressSourceDb {
            source_id: crate::sample_sources::SourceId::from_string("source".to_string()),
            source_root: dir.path().to_path_buf(),
            conn,
        }];
        let cache = Arc::new(RwLock::new(ProgressCache::default()));
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let tx = JobMessageSender::new(tx);
        let stale_before = now - 10;
        let signal = Arc::new(SharedRepaintSignal::default());

        let changed = cleanup_stale_jobs(&mut sources, stale_before, &cache, &tx, &signal);

        assert_eq!(changed, 1);
        let cached = cache.read().unwrap().total_for_sources(std::iter::once(
            &crate::sample_sources::SourceId::from_string("source".to_string()),
        ));
        assert_eq!(cached.failed, 1);
        let message = rx.recv_timeout(Duration::from_millis(50)).unwrap();
        let JobMessage::Analysis(AnalysisJobMessage::Progress { progress, .. }) = message else {
            panic!("unexpected message");
        };
        assert_eq!(progress.failed, 1);
    }

    #[test]
    fn current_progress_all_reuses_cache_without_db_refresh() {
        let dir = TempDir::new().unwrap();
        let conn = db::open_source_db_maintenance(dir.path()).unwrap();
        let source_id = crate::sample_sources::SourceId::from_string("source".to_string());
        let sources = vec![ProgressSourceDb {
            source_id: source_id.clone(),
            source_root: dir.path().to_path_buf(),
            conn,
        }];
        let cache = Arc::new(RwLock::new(ProgressCache::default()));
        cache.write().unwrap().update(
            source_id,
            AnalysisProgress {
                pending: 3,
                running: 1,
                done: 5,
                failed: 2,
                samples_total: 11,
                samples_pending_or_running: 4,
            },
        );

        let progress = current_progress_all(&sources, &cache);

        assert_eq!(progress.pending, 3);
        assert_eq!(progress.running, 1);
        assert_eq!(progress.done, 5);
        assert_eq!(progress.failed, 2);
        assert_eq!(progress.samples_total, 11);
        assert_eq!(progress.samples_pending_or_running, 4);
    }

    #[test]
    fn seed_missing_progress_populates_only_missing_sources() {
        let dir = TempDir::new().unwrap();
        let conn = db::open_source_db_maintenance(dir.path()).unwrap();
        conn.execute(
            "INSERT INTO wav_files (path, file_size, modified_ns, missing)
             VALUES (?1, 1, 0, 0)",
            rusqlite::params!["a.wav"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, status, attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, 0)",
            rusqlite::params!["source::a.wav", "source", "a.wav", db::ANALYZE_SAMPLE_JOB_TYPE],
        )
        .unwrap();
        let source_id = crate::sample_sources::SourceId::from_string("source".to_string());
        let sources = vec![ProgressSourceDb {
            source_id: source_id.clone(),
            source_root: dir.path().to_path_buf(),
            conn,
        }];
        let cache = Arc::new(RwLock::new(ProgressCache::default()));

        assert!(seed_missing_progress(&sources, &cache));
        assert!(!seed_missing_progress(&sources, &cache));

        let progress = cache
            .read()
            .unwrap()
            .total_for_sources(std::iter::once(&source_id));
        assert_eq!(progress.pending, 1);
    }

    #[test]
    fn refresh_sources_reuses_unchanged_source_connections() {
        let config_dir = TempDir::new().unwrap();
        let _config_guard = crate::app_dirs::ConfigBaseGuard::set(config_dir.path().to_path_buf());
        let source_dir = TempDir::new().unwrap();
        let source = crate::sample_sources::SampleSource::new(source_dir.path().to_path_buf());
        crate::sample_sources::SourceDatabase::open(&source.root).expect("seed source db");
        crate::sample_sources::library::save(&crate::sample_sources::library::LibraryState {
            sources: vec![source.clone()],
        })
        .unwrap();
        let mut sources = Vec::<ProgressSourceDb>::new();
        let mut last_refresh = Instant::now() - SOURCE_REFRESH_INTERVAL;

        assert!(refresh_sources(&mut sources, &mut last_refresh, None));
        assert_eq!(sources.len(), 1);
        crate::sample_sources::db::test_reset_source_db_open_total_count(&source.root);
        last_refresh = Instant::now() - SOURCE_REFRESH_INTERVAL;

        assert!(!refresh_sources(&mut sources, &mut last_refresh, None));

        assert_eq!(sources.len(), 1);
        assert_eq!(
            crate::sample_sources::db::test_source_db_open_total_count(&source.root),
            0,
            "periodic progress refresh must reuse the existing UI-read connection"
        );
    }

    #[test]
    fn refresh_sources_drops_tracked_maintenance_session_during_file_op_priority() {
        let config_dir = TempDir::new().unwrap();
        let _config_guard = crate::app_dirs::ConfigBaseGuard::set(config_dir.path().to_path_buf());
        let source_dir = TempDir::new().unwrap();
        let source = crate::sample_sources::SampleSource::new(source_dir.path().to_path_buf());
        crate::sample_sources::SourceDatabase::open(&source.root).expect("seed source db");
        crate::sample_sources::library::save(&crate::sample_sources::library::LibraryState {
            sources: vec![source.clone()],
        })
        .unwrap();
        let mut sources = Vec::<ProgressSourceDb>::new();
        let mut last_refresh = Instant::now() - SOURCE_REFRESH_INTERVAL;
        assert!(refresh_sources(&mut sources, &mut last_refresh, None));
        assert_eq!(sources.len(), 1);
        crate::sample_sources::db::test_reset_source_db_open_total_count(&source.root);

        {
            let _guard = FileOpWritePriorityGuard::new(&source.id);
            last_refresh = Instant::now() - SOURCE_REFRESH_INTERVAL;

            assert!(refresh_sources(&mut sources, &mut last_refresh, None));
            assert!(sources.is_empty());
            assert_eq!(
                crate::sample_sources::db::test_source_db_open_total_count(&source.root),
                0,
                "progress discovery must drop the tracked session without opening a replacement"
            );
        }

        last_refresh = Instant::now() - SOURCE_REFRESH_INTERVAL;
        assert!(refresh_sources(&mut sources, &mut last_refresh, None));
        assert_eq!(sources.len(), 1);
        assert_eq!(
            crate::sample_sources::db::test_source_db_open_total_count(&source.root),
            1,
            "progress discovery should reopen maintenance after file-op priority clears"
        );
    }

    #[test]
    fn wakeup_returns_when_notified() {
        let wakeup = ProgressPollerWakeup::new();
        let mut seen = 0;

        wakeup.notify();

        assert!(wakeup.wait_for(&mut seen, Duration::from_millis(1)));
        assert!(!wakeup.wait_for(&mut seen, Duration::from_millis(1)));
    }
}
