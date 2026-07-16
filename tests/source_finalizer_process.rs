//! Process-boundary coverage for cancellable native similarity finalization.

use std::process::Command;

use wavecrate::sample_sources::{SampleSource, SourceDatabase, SourceId};

#[test]
fn internal_similarity_finalizer_runs_to_completion_without_starting_the_gui() {
    let directory = tempfile::tempdir().expect("temporary similarity source");
    let source = SampleSource::new_with_id(
        SourceId::from_string("process-finalizer"),
        directory.path().to_path_buf(),
    );
    let database =
        SourceDatabase::open_for_source_write(directory.path()).expect("create source database");
    database
        .set_metadata("last_scan_completed_at", "100")
        .expect("seed completed scan timestamp");
    drop(database);
    let source_json = serde_json::to_string(&source).expect("encode source descriptor");
    let fence_json = r#"{"LegacyPathsRevision":0}"#;

    let output = Command::new(env!("CARGO_BIN_EXE_wavecrate"))
        .arg("--wavecrate-internal-similarity-finalizer-v1")
        .arg(source_json)
        .arg(fence_json)
        .output()
        .expect("run internal finalizer process");

    assert!(
        output.status.success(),
        "internal finalizer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "true");
    assert_eq!(
        source
            .open_db()
            .expect("reopen finalized source")
            .get_metadata("last_similarity_prep_scan_at")
            .expect("read finalizer timestamp")
            .as_deref(),
        Some("100")
    );
}
