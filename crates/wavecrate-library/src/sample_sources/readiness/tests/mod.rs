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

fn replace(connection: &mut Connection, generation: i64, targets: &[ReadinessTarget]) {
    replace_readiness_targets(
        connection,
        SOURCE_ID,
        generation,
        generation + 100,
        SourceAvailability::Active,
        targets,
        100,
    )
    .expect("replace targets");
}
