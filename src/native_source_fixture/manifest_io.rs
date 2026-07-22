use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{FixtureFileManifest, topology::GeneratedAudio};

pub(super) fn file_manifest(
    source_id: &str,
    relative_path: &str,
    path: &Path,
    supported_audio: bool,
    audio: Option<&GeneratedAudio>,
) -> Result<FixtureFileManifest, String> {
    Ok(FixtureFileManifest {
        source_id: source_id.to_owned(),
        relative_path: relative_path.replace('\\', "/"),
        sha256: sha256(path)?,
        supported_audio,
        channels: audio.map(|spec| spec.channels),
        sample_rate: audio.map(|spec| spec.sample_rate),
        frames: audio.map(|spec| spec.frames),
    })
}

pub(super) fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read fixture input {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize fixture manifest: {error}"))?;
    bytes.push(b'\n');
    fs::write(path, bytes)
        .map_err(|error| format!("write fixture manifest {}: {error}", path.display()))
}
