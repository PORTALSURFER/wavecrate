use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use super::super::{Rating, SourceDbError};

/// File operation kinds tracked in the per-source journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOpKind {
    /// Moving a file between sources.
    Move,
    /// Copying a file into a source.
    Copy,
}

impl FileOpKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            FileOpKind::Move => "move",
            FileOpKind::Copy => "copy",
        }
    }

    pub(super) fn from_str(value: &str) -> Option<Self> {
        match value {
            "move" => Some(FileOpKind::Move),
            "copy" => Some(FileOpKind::Copy),
            _ => None,
        }
    }
}

/// Persistent journal stages for file operations that need crash recovery.
///
/// Recovery treats the enum as an append-only lifecycle:
/// `Intent -> Staged -> TargetDb -> SourceDb`.
///
/// The stage is descriptive rather than authoritative; reconcile still inspects
/// the actual filesystem and database state so startup recovery remains
/// idempotent after partial writes or repeated runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOpStage {
    /// Intent recorded before any filesystem mutations.
    Intent,
    /// File moved/copied into staging location.
    Staged,
    /// Target database updated.
    TargetDb,
    /// Source database updated (move only).
    SourceDb,
}

impl FileOpStage {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            FileOpStage::Intent => "intent",
            FileOpStage::Staged => "staged",
            FileOpStage::TargetDb => "target_db",
            FileOpStage::SourceDb => "source_db",
        }
    }

    pub(super) fn from_str(value: &str) -> Option<Self> {
        match value {
            "intent" => Some(FileOpStage::Intent),
            "staged" => Some(FileOpStage::Staged),
            "target_db" => Some(FileOpStage::TargetDb),
            "source_db" => Some(FileOpStage::SourceDb),
            _ => None,
        }
    }
}

/// Durable journal entry used to reconcile file and database state after crashes.
#[derive(Debug, Clone)]
pub(crate) struct FileOpJournalEntry {
    pub(crate) id: String,
    pub(crate) kind: FileOpKind,
    pub(crate) stage: FileOpStage,
    pub(crate) source_root: Option<PathBuf>,
    pub(crate) source_relative: Option<PathBuf>,
    pub(crate) target_relative: PathBuf,
    pub(crate) staged_relative: Option<PathBuf>,
    pub(crate) file_size: Option<u64>,
    pub(crate) modified_ns: Option<i64>,
    pub(crate) tag: Option<Rating>,
    pub(crate) looped: Option<bool>,
    pub(crate) last_played_at: Option<i64>,
    pub(crate) created_at: i64,
}

/// Initialization payload for creating move journal entries without wide signatures.
#[derive(Debug, Clone)]
pub(crate) struct MoveJournalEntryInit {
    pub(crate) source_root: PathBuf,
    pub(crate) source_relative: PathBuf,
    pub(crate) target_relative: PathBuf,
    pub(crate) staged_relative: PathBuf,
    pub(crate) tag: Rating,
    pub(crate) looped: bool,
    pub(crate) last_played_at: Option<i64>,
}

impl FileOpJournalEntry {
    /// Build a new journal entry for a move operation.
    pub(crate) fn new_move(id: String, init: MoveJournalEntryInit) -> Result<Self, SourceDbError> {
        Ok(Self {
            id,
            kind: FileOpKind::Move,
            stage: FileOpStage::Intent,
            source_root: Some(init.source_root),
            source_relative: Some(init.source_relative),
            target_relative: init.target_relative,
            staged_relative: Some(init.staged_relative),
            file_size: None,
            modified_ns: None,
            tag: Some(init.tag),
            looped: Some(init.looped),
            last_played_at: init.last_played_at,
            created_at: now_epoch_seconds()?,
        })
    }

    /// Build a new journal entry for a copy operation.
    pub(crate) fn new_copy(
        id: String,
        target_relative: PathBuf,
        staged_relative: PathBuf,
    ) -> Result<Self, SourceDbError> {
        Ok(Self {
            id,
            kind: FileOpKind::Copy,
            stage: FileOpStage::Intent,
            source_root: None,
            source_relative: None,
            target_relative,
            staged_relative: Some(staged_relative),
            file_size: None,
            modified_ns: None,
            tag: None,
            looped: None,
            last_played_at: None,
            created_at: now_epoch_seconds()?,
        })
    }
}

/// Generate a unique identifier for a pending file operation.
pub(crate) fn new_op_id() -> String {
    Uuid::new_v4().to_string()
}

/// Create a deterministic staging path that lives beside the final destination.
pub(crate) fn staged_relative_for_target(
    target_relative: &Path,
    op_id: &str,
) -> Result<PathBuf, SourceDbError> {
    let file_name = target_relative
        .file_name()
        .ok_or_else(|| SourceDbError::InvalidRelativePath(target_relative.to_path_buf()))?;
    let staged_name = format!("{}.sempal_pending_{}", file_name.to_string_lossy(), op_id);
    Ok(target_relative.with_file_name(staged_name))
}

fn now_epoch_seconds() -> Result<i64, SourceDbError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SourceDbError::Unexpected)?;
    Ok(now.as_secs() as i64)
}
