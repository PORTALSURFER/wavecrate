use super::*;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{SourceDatabase, is_supported_audio};

pub(super) struct SourcePasteContext {
    pub(super) source_id: SourceId,
    source_root: PathBuf,
    target_folder: PathBuf,
    target_root: PathBuf,
    pub(super) db: SourceDatabase,
}

impl SourcePasteContext {
    pub(super) fn new(job: SourceClipboardPasteJob) -> Result<Self, String> {
        validate_relative_folder_path(&job.target_folder)?;
        if !job.source_root.is_dir() {
            return Err("Source folder is not available".to_string());
        }
        let target_root = job.source_root.join(&job.target_folder);
        if !target_root.exists() {
            std::fs::create_dir_all(&target_root).map_err(|err| {
                format!("Failed to create folder {}: {err}", target_root.display())
            })?;
        } else if !target_root.is_dir() {
            return Err(format!(
                "Target folder is not a directory: {}",
                target_root.display()
            ));
        }
        let db = SourceDatabase::open(&job.source_root)
            .map_err(|err| format!("Failed to open source DB: {err}"))?;
        Ok(Self {
            source_id: job.source_id,
            source_root: job.source_root,
            target_folder: job.target_folder,
            target_root,
            db,
        })
    }
}

pub(super) struct PreparedSourcePaste {
    pub(super) source_path: PathBuf,
    pub(super) relative: PathBuf,
    pub(super) staged_relative: PathBuf,
    pub(super) staged_absolute: PathBuf,
    pub(super) absolute: PathBuf,
    pub(super) op_id: String,
}

/// Validate one incoming path and derive its target + staging paths.
pub(super) fn prepare_source_paste(
    context: &SourcePasteContext,
    path: &Path,
) -> Result<Option<PreparedSourcePaste>, String> {
    if !path.is_file() || !is_supported_audio(path) {
        return Ok(None);
    }
    let relative_name = unique_destination_name(&context.target_root, path)?;
    let relative = if context.target_folder.as_os_str().is_empty() {
        relative_name
    } else {
        context.target_folder.join(relative_name)
    };
    let op_id = file_ops_journal::new_op_id();
    let staged_relative = file_ops_journal::staged_relative_for_target(&relative, &op_id)
        .map_err(|err| format!("Failed to build staging path: {err}"))?;
    Ok(Some(PreparedSourcePaste {
        source_path: path.to_path_buf(),
        staged_absolute: context.source_root.join(&staged_relative),
        absolute: context.source_root.join(&relative),
        relative,
        staged_relative,
        op_id,
    }))
}
