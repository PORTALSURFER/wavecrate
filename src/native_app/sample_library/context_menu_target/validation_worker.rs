use std::path::Path;

use super::{BrowserContextTargetKind, missing_target_message};

pub(super) fn validate_open_target(
    kind: &BrowserContextTargetKind,
    path: &Path,
) -> Result<(), String> {
    let exists = path
        .try_exists()
        .map_err(|error| format!("Cannot access {}: {error}", path.display()))?;
    if !exists {
        return Err(missing_target_message(kind).to_string());
    }

    match kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder if !path.is_dir() => {
            Err(format!("Not a folder: {}", path.display()))
        }
        BrowserContextTargetKind::Sample if !path.is_file() => {
            Err(format!("Not a file: {}", path.display()))
        }
        BrowserContextTargetKind::Collection | BrowserContextTargetKind::MetadataTag => {
            Err(missing_target_message(kind).to_string())
        }
        BrowserContextTargetKind::Source
        | BrowserContextTargetKind::Folder
        | BrowserContextTargetKind::Sample => Ok(()),
    }
}
