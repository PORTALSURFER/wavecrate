//! Filesystem persistence helpers for run-contract artifacts.

use super::artifacts::{RunContractEvent, RunContractManifest};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
use tracing::error;

pub(super) fn write_manifest(path: &Path, run_id: &str, manifest: &RunContractManifest) {
    let Ok(serialized) = serde_json::to_string_pretty(manifest) else {
        error!("[run_contract] failed to serialize run manifest for run_id {run_id}");
        return;
    };

    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        error!(
            "[run_contract] failed to ensure manifest directory {}: {err}",
            parent.display()
        );
        return;
    }

    if let Err(err) = fs::write(path, serialized) {
        error!("[run_contract] failed to write {}: {err}", path.display());
    }
}

pub(super) fn append_event(path: &Path, run_id: &str, event: &RunContractEvent) {
    let Ok(serialized) = serde_json::to_string(event) else {
        error!("[run_contract] failed to serialize run contract event for run_id {run_id}");
        return;
    };

    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        error!(
            "[run_contract] failed to ensure artifact directory {}: {err}",
            parent.display()
        );
        return;
    }

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| writeln!(file, "{serialized}"))
    {
        Ok(()) => {}
        Err(err) => {
            error!("[run_contract] failed to write {}: {err}", path.display());
        }
    }
}
