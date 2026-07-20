use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::analysis_jobs::AnalysisJobMessage;
use crate::app::controller::state::cache::{FeatureCache, FeatureCacheKey};
use crate::app::controller::test_support::{dummy_controller, write_test_wav};
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::scanner::{
    ChangedSample, RenamedSample, ScanError, ScanStats, UpdatedSample,
};
use crate::sample_sources::{Rating, ScanMode, SourceId};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

fn wait_for_analysis_message<F>(controller: &mut AppController, predicate: F) -> AnalysisJobMessage
where
    F: Fn(&AnalysisJobMessage) -> bool,
{
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message)) if predicate(&message) => return message,
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("expected analysis message, got {err:?}"),
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for analysis message"
        );
    }
}

fn assert_no_analysis_message(controller: &mut AppController) {
    let deadline = Instant::now() + Duration::from_millis(150);
    loop {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message)) => {
                panic!("unexpected analysis message: {message:?}");
            }
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(err) => panic!("unexpected receive error: {err:?}"),
        }
    }
}

fn assert_no_analysis_jobs_inserted(source: &SampleSource) {
    let conn =
        crate::sample_sources::SourceDatabase::open_connection_for_background_job(&source.root)
            .expect("open source db");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM analysis_jobs
             WHERE job_type IN (?1, ?2)",
            rusqlite::params!["wav_metadata_v1", "embedding_backfill_v1",],
            |row| row.get(0),
        )
        .expect("count analysis jobs");
    assert_eq!(count, 0, "scan completion inserted analysis jobs");
}

fn scan_result(
    source_id: SourceId,
    mode: ScanMode,
    kind: ScanKind,
    result: Result<ScanStats, ScanError>,
) -> ScanResult {
    ScanResult {
        source_id,
        mode,
        kind,
        result,
    }
}

fn changed_sample(relative_path: &str) -> ChangedSample {
    ChangedSample {
        relative_path: PathBuf::from(relative_path),
        file_size: 8,
        modified_ns: 42,
        content_hash: "hash-v2".to_string(),
    }
}

fn updated_sample(relative_path: &str) -> UpdatedSample {
    UpdatedSample {
        relative_path: PathBuf::from(relative_path),
        file_size: 8,
        modified_ns: 42,
        content_hash: None,
    }
}

fn renamed_sample(old_relative_path: &str, new_relative_path: &str) -> RenamedSample {
    RenamedSample {
        old_relative_path: PathBuf::from(old_relative_path),
        new_relative_path: PathBuf::from(new_relative_path),
        file_size: 8,
        modified_ns: 42,
        content_hash: None,
    }
}

mod analysis_suppression;
mod progress_reporting;
mod result_application;
