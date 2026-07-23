use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{Event, FailureSnapshot};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ReplayArtifact {
    pub(super) schema_version: u32,
    pub(super) seed: u64,
    pub(super) fixture: String,
    pub(super) fixture_version: u32,
    pub(super) fixture_seed: u64,
    pub(super) events: Vec<Event>,
    pub(super) failure: FailureSnapshot,
    pub(super) replay_command: String,
}

pub(super) fn write_failure_artifact(
    seed: u64,
    events: Vec<Event>,
    failure: FailureSnapshot,
) -> Result<PathBuf, String> {
    let root = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"))
        .join("source-state-machine-failures");
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "create replay artifact directory {}: {error}",
            root.display()
        )
    })?;
    let path = root.join(format!("seed-{seed:016x}.json"));
    let replay_command = format!(
        "WAVECRATE_SOURCE_STATE_MACHINE_REPLAY={} cargo test -p wavecrate --lib \
         source_processing_seeded_state_machine_replay -- --ignored --nocapture",
        path.display()
    );
    let artifact = ReplayArtifact {
        schema_version: 1,
        seed,
        fixture: String::from("small-multi-source"),
        fixture_version: crate::native_source_fixture::FIXTURE_VERSION,
        fixture_seed: 0x5741_5645_4352_4154,
        events,
        failure,
        replay_command,
    };
    let bytes = serde_json::to_vec_pretty(&artifact)
        .map_err(|error| format!("serialize replay artifact: {error}"))?;
    fs::write(&path, bytes)
        .map_err(|error| format!("write replay artifact {}: {error}", path.display()))?;
    Ok(path)
}

pub(super) fn read_replay(value: &str) -> Result<(u64, Vec<Event>), String> {
    if let Ok(seed) = value.parse::<u64>() {
        return Ok((seed, super::generate(seed, super::NORMAL_SEQUENCE_LENGTH)));
    }
    let path = Path::new(value);
    let bytes = fs::read(path)
        .map_err(|error| format!("read replay artifact {}: {error}", path.display()))?;
    let artifact: ReplayArtifact = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse replay artifact {}: {error}", path.display()))?;
    if artifact.schema_version != 1 {
        return Err(format!(
            "unsupported source state-machine artifact schema {}",
            artifact.schema_version
        ));
    }
    Ok((artifact.seed, artifact.events))
}
