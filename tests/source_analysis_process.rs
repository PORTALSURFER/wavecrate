//! Process-boundary coverage for cancellable native source analysis.

use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use rusqlite::params;
use serde_json::{Value, json};
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceId,
    readiness::{
        ReadinessEligibility, ReadinessStage, ReadinessTarget, ReadinessWorkMutationOutcome,
        SourceAvailability, cancel_readiness_work, claim_readiness_target,
        persist_readiness_deficits, reconcile_readiness, replace_readiness_targets,
    },
};

const INTERNAL_SOURCE_ANALYSIS_ARG: &str = "--wavecrate-internal-source-analysis-v1";
const SOURCE_GENERATION: i64 = 1;

#[test]
fn internal_feature_and_embedding_workers_complete_without_starting_the_gui() {
    let fixture = SourceAnalysisFixture::new("process-analysis-complete", "short.wav", 2, 8_000);

    let feature = run_internal_analysis(&fixture.feature_request());
    assert_eq!(feature["produced"], true);

    let embedding = run_internal_analysis(&fixture.embedding_request());
    assert_eq!(embedding["produced"], true);
}

#[test]
fn long_running_feature_worker_is_killable_and_its_claim_is_reclaimable() {
    let mut fixture = SourceAnalysisFixture::new("process-analysis-cancel", "long.wav", 300, 8_000);
    let target = fixture.install_feature_readiness_target();
    let claim = claim_readiness_target(&mut fixture.connection, &target, 10, 300)
        .expect("claim long-running feature target")
        .expect("feature target should be pending");

    let request_json = serde_json::to_string(&fixture.feature_request()).expect("encode request");
    let mut child = Command::new(env!("CARGO_BIN_EXE_wavecrate"))
        .arg(INTERNAL_SOURCE_ANALYSIS_ARG)
        .arg(request_json)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start long-running feature worker");

    thread::sleep(Duration::from_millis(150));
    assert!(
        child.try_wait().expect("poll feature worker").is_none(),
        "the long-running feature worker completed before cancellation could be exercised"
    );

    let cancellation_started = Instant::now();
    child.kill().expect("terminate feature worker");
    let status = child.wait().expect("join terminated feature worker");
    assert!(
        !status.success(),
        "terminated worker unexpectedly succeeded"
    );
    assert!(
        cancellation_started.elapsed() < Duration::from_secs(1),
        "terminating and joining the feature worker took {:?}",
        cancellation_started.elapsed()
    );

    assert_eq!(
        cancel_readiness_work(
            &mut fixture.connection,
            &claim,
            "test playback preemption",
            11,
        )
        .expect("release cancelled readiness claim"),
        ReadinessWorkMutationOutcome::Recorded
    );
    assert!(
        claim_readiness_target(&mut fixture.connection, &target, 12, 300)
            .expect("reclaim cancelled feature target")
            .is_some(),
        "cancelled readiness work should be immediately reclaimable"
    );
    assert_eq!(
        fixture
            .connection
            .query_row(
                "SELECT COUNT(*) FROM source_readiness_artifacts
                 WHERE source_id = ?1 AND scope_id = ?2 AND stage = 'analysis_features'",
                params![fixture.source.id.as_str(), fixture.file_identity],
                |row| row.get::<_, i64>(0),
            )
            .expect("read feature readiness artifacts"),
        0,
        "a killed worker must not publish readiness completion"
    );
}

struct SourceAnalysisFixture {
    _directory: tempfile::TempDir,
    source: SampleSource,
    connection: rusqlite::Connection,
    relative_path: String,
    file_identity: String,
    content_hash: String,
}

impl SourceAnalysisFixture {
    fn new(
        source_id: &str,
        relative_path: &str,
        duration_seconds: usize,
        sample_rate: u32,
    ) -> Self {
        let directory = tempfile::tempdir().expect("temporary analysis source");
        let source = SampleSource::new_with_id(
            SourceId::from_string(source_id),
            directory.path().to_path_buf(),
        );
        let absolute_path = directory.path().join(relative_path);
        write_test_wav(&absolute_path, duration_seconds, sample_rate);
        let metadata = std::fs::metadata(&absolute_path).expect("read wav metadata");
        let file_identity = format!("identity-{source_id}");
        let content_hash =
            blake3::hash(&std::fs::read(&absolute_path).expect("read wav for content hash"))
                .to_hex()
                .to_string();
        let connection = SourceDatabase::open_connection(directory.path()).expect("open source db");
        connection
            .execute(
                "INSERT INTO wav_files (
                    path, file_size, modified_ns, content_hash, extension, missing, file_identity
                 ) VALUES (?1, ?2, 1, ?3, 'wav', 0, ?4)",
                params![
                    relative_path,
                    i64::try_from(metadata.len()).expect("wav size fits i64"),
                    content_hash,
                    file_identity,
                ],
            )
            .expect("seed wav manifest");
        Self {
            _directory: directory,
            source,
            connection,
            relative_path: relative_path.to_string(),
            file_identity,
            content_hash,
        }
    }

    fn feature_request(&self) -> Value {
        json!({
            "source": self.source,
            "task": {
                "ReadinessFeature": {
                    "relative_path": self.relative_path,
                    "content_hash": self.content_hash,
                    "analysis_version": wavecrate_analysis::analysis_version(),
                }
            }
        })
    }

    fn embedding_request(&self) -> Value {
        json!({
            "source": self.source,
            "task": {
                "ReadinessEmbedding": {
                    "relative_path": self.relative_path,
                    "content_hash": self.content_hash,
                    "analysis_version": wavecrate_analysis::analysis_version(),
                }
            }
        })
    }

    fn install_feature_readiness_target(&mut self) -> ReadinessTarget {
        let feature_target = ReadinessTarget::file(
            self.source.id.as_str(),
            self.file_identity.as_str(),
            self.relative_path.as_str(),
            ReadinessStage::AnalysisFeatures,
            wavecrate_analysis::analysis_version(),
            SOURCE_GENERATION,
            self.content_hash.as_str(),
        );
        let mut targets = Vec::new();
        for stage in [
            ReadinessStage::IndexedIdentity,
            ReadinessStage::PlaybackSummary,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            let mut target = feature_target.clone();
            target.stage = stage;
            if stage != ReadinessStage::AnalysisFeatures {
                target.required_version = "test-unsupported-v1".to_string();
                target.eligibility = ReadinessEligibility::Unsupported;
            }
            targets.push(target);
        }
        targets.push(
            ReadinessTarget::source(
                self.source.id.as_str(),
                ReadinessStage::SimilarityLayout,
                "test-unsupported-v1",
                SOURCE_GENERATION,
                "test-membership-v1",
            )
            .with_eligibility(ReadinessEligibility::Unsupported),
        );
        replace_readiness_targets(
            &mut self.connection,
            self.source.id.as_str(),
            SOURCE_GENERATION,
            1,
            SourceAvailability::Active,
            &targets,
            1,
        )
        .expect("install readiness targets");
        let snapshot = reconcile_readiness(&self.connection, self.source.id.as_str(), 2)
            .expect("reconcile feature readiness");
        assert_eq!(snapshot.deficits.len(), 1);
        assert_eq!(
            persist_readiness_deficits(&mut self.connection, &snapshot.deficits, 2)
                .expect("persist feature readiness deficit"),
            1
        );
        feature_target
    }
}

fn run_internal_analysis(request: &Value) -> Value {
    let request_json = serde_json::to_string(request).expect("encode analysis request");
    let output = Command::new(env!("CARGO_BIN_EXE_wavecrate"))
        .arg(INTERNAL_SOURCE_ANALYSIS_ARG)
        .arg(request_json)
        .output()
        .expect("run internal source analysis process");
    assert!(
        output.status.success(),
        "internal source analysis failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("decode internal source analysis result")
}

fn write_test_wav(path: &std::path::Path, duration_seconds: usize, sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create test wav");
    let sample_count = duration_seconds
        .checked_mul(usize::try_from(sample_rate).expect("sample rate fits usize"))
        .expect("sample count fits usize");
    for sample_index in 0..sample_count {
        let phase = (sample_index % 200) as f32 / 200.0;
        let sample = ((phase * std::f32::consts::TAU).sin() * 16_000.0) as i16;
        writer.write_sample(sample).expect("write test sample");
    }
    writer.finalize().expect("finalize test wav");
}
