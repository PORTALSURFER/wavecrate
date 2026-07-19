//! Process-boundary coverage for durable readiness lease ownership and recovery.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

use wavecrate_library::sample_sources::{
    SourceDatabase,
    readiness::{
        ArtifactPublishOutcome, ClaimedReadinessWork, ReadinessClaimOrigin, ReadinessEligibility,
        ReadinessFailureClassification, ReadinessFailureOutcome, ReadinessRetryPolicy,
        ReadinessStage, ReadinessStore, ReadinessTarget, ReadinessTargetPublication,
        SourceAvailability,
    },
};

const SOURCE_ID: &str = "process-lease-source";
const CLAIM_NOW_ENV: &str = "WAVECRATE_READINESS_CLAIM_NOW";
const SOURCE_ROOT_ENV: &str = "WAVECRATE_READINESS_SOURCE_ROOT";
const RESULT_PATH_ENV: &str = "WAVECRATE_READINESS_RESULT_PATH";
const START_PATH_ENV: &str = "WAVECRATE_READINESS_START_PATH";

fn analysis_target() -> ReadinessTarget {
    ReadinessTarget::file(
        SOURCE_ID,
        "process-file-identity",
        "process.wav",
        ReadinessStage::AnalysisFeatures,
        "analysis-v1",
        1,
        "process-content-generation",
    )
}

fn seed_readiness_work(root: &Path) {
    let mut connection = SourceDatabase::open_connection(root).expect("create source database");
    connection
        .execute(
            "INSERT INTO wav_files (
                path, file_size, modified_ns, extension, missing, file_identity
             ) VALUES ('process.wav', 1, 1, 'wav', 0, 'process-file-identity')",
            [],
        )
        .expect("seed source manifest");
    let analysis = analysis_target();
    let targets = vec![
        ReadinessTarget::file(
            SOURCE_ID,
            "process-file-identity",
            "process.wav",
            ReadinessStage::IndexedIdentity,
            "manifest-v1",
            1,
            "process-content-generation",
        )
        .with_eligibility(ReadinessEligibility::Unsupported),
        analysis.clone(),
        ReadinessTarget::file(
            SOURCE_ID,
            "process-file-identity",
            "process.wav",
            ReadinessStage::EmbeddingAspects,
            "embedding-v1",
            1,
            "process-content-generation",
        )
        .with_eligibility(ReadinessEligibility::Unsupported),
        ReadinessTarget::source(
            SOURCE_ID,
            ReadinessStage::SimilarityLayout,
            "layout-v1",
            1,
            "membership-v1",
        )
        .with_eligibility(ReadinessEligibility::Unsupported),
    ];
    let publication = ReadinessTargetPublication::new(
        SOURCE_ID,
        1,
        1,
        SourceAvailability::Active,
        "readiness-lease-process-test-v1",
        &targets,
        10,
    );
    ReadinessStore::new(&mut connection)
        .publish_targets(&publication)
        .expect("publish readiness targets");
    let snapshot = ReadinessStore::new(&mut connection)
        .reconcile(SOURCE_ID, 10)
        .expect("reconcile readiness");
    ReadinessStore::new(&mut connection)
        .persist_deficits(&snapshot.deficits, 10)
        .expect("persist readiness work");
}

fn claim_command(root: &Path, now: i64, result_path: &Path, start_path: Option<&Path>) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("current test executable"));
    command
        .arg("--exact")
        .arg("readiness_claim_process_helper")
        .arg("--ignored")
        .arg("--nocapture")
        .env(SOURCE_ROOT_ENV, root)
        .env(CLAIM_NOW_ENV, now.to_string())
        .env(RESULT_PATH_ENV, result_path);
    if let Some(start_path) = start_path {
        command.env(START_PATH_ENV, start_path);
    }
    command
}

fn assert_process_succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "claim process failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn crashed_process_lease_is_protected_until_expiry_and_recovered_once() {
    let root = tempfile::tempdir().expect("source root");
    seed_readiness_work(root.path());

    let first_result = root.path().join("first-result");
    let first = claim_command(root.path(), 10, &first_result, None)
        .output()
        .expect("run original claimant");
    assert_process_succeeded(&first);
    assert_eq!(
        fs::read_to_string(&first_result).expect("read original result"),
        "claimed:1:pending:20"
    );

    let protected_result = root.path().join("protected-result");
    let protected = claim_command(root.path(), 19, &protected_result, None)
        .output()
        .expect("run protected claimant");
    assert_process_succeeded(&protected);
    assert_eq!(
        fs::read_to_string(&protected_result).expect("read protected result"),
        "none"
    );

    let start_path = root.path().join("start-expired-claimants");
    let left_result = root.path().join("left-result");
    let right_result = root.path().join("right-result");
    let left = claim_command(root.path(), 20, &left_result, Some(&start_path))
        .spawn()
        .expect("spawn left claimant");
    let right = claim_command(root.path(), 20, &right_result, Some(&start_path))
        .spawn()
        .expect("spawn right claimant");
    fs::write(&start_path, b"start").expect("release expired claimants");
    let left_output = left.wait_with_output().expect("wait for left claimant");
    let right_output = right.wait_with_output().expect("wait for right claimant");
    assert_process_succeeded(&left_output);
    assert_process_succeeded(&right_output);
    let results = [
        fs::read_to_string(left_result).expect("read left result"),
        fs::read_to_string(right_result).expect("read right result"),
    ];
    assert_eq!(
        results
            .iter()
            .filter(|result| result.as_str() == "none")
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| result.as_str() == "claimed:2:expired_lease:30")
            .count(),
        1
    );

    let original = ClaimedReadinessWork {
        target: analysis_target(),
        claim_generation: 1,
        failure_attempts: 0,
        lease_expires_at: 20,
        origin: ReadinessClaimOrigin::Pending,
    };
    let mut connection = SourceDatabase::open_connection(root.path()).expect("reopen source db");
    assert_eq!(
        ReadinessStore::new(&mut connection)
            .complete(&original, 21)
            .expect("reject stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );
    let policy = ReadinessRetryPolicy::new(1, 10, 2).expect("retry policy");
    assert_eq!(
        ReadinessStore::new(&mut connection)
            .fail(
                &original,
                ReadinessFailureClassification::Retryable,
                "stale_process",
                "original process reported failure after recovery",
                21,
                policy,
            )
            .expect("reject stale failure"),
        ReadinessFailureOutcome::RejectedStale
    );
}

#[test]
#[ignore = "spawned by the process-boundary lease test"]
fn readiness_claim_process_helper() {
    let root = PathBuf::from(std::env::var_os(SOURCE_ROOT_ENV).expect("source root env"));
    let now = std::env::var(CLAIM_NOW_ENV)
        .expect("claim time env")
        .parse::<i64>()
        .expect("numeric claim time");
    let result_path = PathBuf::from(std::env::var_os(RESULT_PATH_ENV).expect("result path env"));
    if let Some(start_path) = std::env::var_os(START_PATH_ENV).map(PathBuf::from) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !start_path.exists() {
            assert!(
                Instant::now() < deadline,
                "timed out waiting for claim start"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
    let mut connection = SourceDatabase::open_connection(&root).expect("open claimant database");
    let result = ReadinessStore::new(&mut connection)
        .claim(&analysis_target(), now, 10)
        .expect("claim readiness work")
        .map_or_else(
            || "none".to_string(),
            |claim| {
                format!(
                    "claimed:{}:{}:{}",
                    claim.claim_generation(),
                    claim.origin().as_str(),
                    claim.lease_expires_at()
                )
            },
        );
    fs::write(result_path, result).expect("write claim result");
}
