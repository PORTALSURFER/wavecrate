//! Process-boundary coverage for current-contract native similarity finalization.

use std::process::Command;

use wavecrate::sample_sources::{SampleSource, SourceDatabase, SourceId};

#[test]
fn internal_similarity_finalizer_rejects_a_noncurrent_contract_without_starting_the_gui() {
    let directory = tempfile::tempdir().expect("temporary similarity source");
    let source = SampleSource::new_with_id(
        SourceId::from_string("process-finalizer"),
        directory.path().to_path_buf(),
    );
    SourceDatabase::open_for_source_write(directory.path()).expect("create source database");
    let source_json = serde_json::to_string(&source).expect("encode source descriptor");
    let fence_json = r#"{
        "source_id":"process-finalizer",
        "source_generation":0,
        "membership_generation":"missing-generation",
        "artifact_version":"missing-contract"
    }"#;

    let output = Command::new(env!("CARGO_BIN_EXE_wavecrate"))
        .arg("--wavecrate-internal-similarity-finalizer-v1")
        .arg(source_json)
        .arg(fence_json)
        .output()
        .expect("run internal finalizer process");

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("does not match the active source contract")
    );
    assert_eq!(
        source
            .open_db()
            .expect("reopen finalized source")
            .get_metadata("last_similarity_prep_scan_at")
            .expect("read retired finalizer timestamp"),
        None,
        "the current finalizer must not publish legacy timestamp readiness"
    );
}
