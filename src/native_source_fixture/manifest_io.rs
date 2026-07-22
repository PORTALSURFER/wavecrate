use std::{fs, path::Path};

use serde::Serialize;
use sha2::{Digest, Sha256};

pub(super) fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("read fixture input {}: {error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize fixture manifest: {error}"))?;
    bytes.push(b'\n');
    fs::write(path, bytes)
        .map_err(|error| format!("write fixture manifest {}: {error}", path.display()))
}
