use rusqlite::Connection;

use super::*;
use crate::sample_sources::{SourceDatabase, SourceDatabaseConnectionRole};

mod classification;
mod persistence;

const SOURCE_ID: &str = "source-a";

fn open_fixture() -> (tempfile::TempDir, Connection) {
    let root = tempfile::tempdir().expect("source root");
    let connection = SourceDatabase::open_connection(root.path()).expect("source db");
    (root, connection)
}

fn file_target(identity: &str, stage: ReadinessStage, generation: i64) -> ReadinessTarget {
    ReadinessTarget::file(
        SOURCE_ID,
        identity,
        format!("Pack/{identity}.wav"),
        stage,
        "v1",
        generation,
        format!("content-{identity}-{generation}"),
    )
}

fn complete_targets(generation: i64, targets: &[ReadinessTarget]) -> Vec<ReadinessTarget> {
    let mut complete = targets.to_vec();
    let file_targets = targets
        .iter()
        .filter(|target| target.scope_kind == ReadinessScopeKind::File)
        .cloned()
        .collect::<Vec<_>>();
    for seed in file_targets {
        for stage in [
            ReadinessStage::IndexedIdentity,
            ReadinessStage::PlaybackSummary,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            if complete.iter().any(|target| {
                target.scope_kind == ReadinessScopeKind::File
                    && target.scope_id == seed.scope_id
                    && target.stage == stage
            }) {
                continue;
            }
            let mut terminal = seed.clone();
            terminal.stage = stage;
            terminal.required_version = "test-unsupported-v1".to_string();
            terminal.eligibility = if seed.eligibility == ReadinessEligibility::Deleted {
                ReadinessEligibility::Deleted
            } else {
                ReadinessEligibility::Unsupported
            };
            complete.push(terminal);
        }
    }
    if !complete
        .iter()
        .any(|target| target.stage == ReadinessStage::SimilarityLayout)
    {
        complete.push(
            ReadinessTarget::source(
                SOURCE_ID,
                ReadinessStage::SimilarityLayout,
                "test-layout-v1",
                generation,
                format!("test-membership-{generation}"),
            )
            .with_eligibility(ReadinessEligibility::Unsupported),
        );
    }
    complete
}

fn sync_manifest(connection: &Connection, targets: &[ReadinessTarget]) {
    connection
        .execute("DELETE FROM wav_files", [])
        .expect("clear test manifest");
    let mut identities = std::collections::BTreeSet::new();
    for target in targets.iter().filter(|target| {
        target.scope_kind == ReadinessScopeKind::File
            && target.stage == ReadinessStage::IndexedIdentity
            && target.eligibility != ReadinessEligibility::Deleted
    }) {
        if !identities.insert(target.scope_id.clone()) {
            continue;
        }
        connection
            .execute(
                "INSERT INTO wav_files (
                    path, file_size, modified_ns, extension, missing, file_identity
                 ) VALUES (?1, 1, 1, 'wav', 0, ?2)",
                rusqlite::params![target.relative_path, target.scope_id],
            )
            .expect("seed test manifest identity");
    }
}

fn replace(connection: &mut Connection, generation: i64, targets: &[ReadinessTarget]) {
    let targets = complete_targets(generation, targets);
    sync_manifest(connection, &targets);
    replace_readiness_targets(
        connection,
        SOURCE_ID,
        generation,
        generation + 100,
        SourceAvailability::Active,
        &targets,
        100,
    )
    .expect("replace targets");
}

fn entry_for<'a>(
    snapshot: &'a ReadinessSnapshot,
    scope_id: &str,
    stage: ReadinessStage,
) -> &'a ReadinessEntry {
    snapshot
        .entries
        .iter()
        .find(|entry| entry.target.scope_id == scope_id && entry.target.stage == stage)
        .expect("readiness entry")
}
