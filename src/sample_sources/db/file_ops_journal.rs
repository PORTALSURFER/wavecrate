//! Durable per-source file-operation journal and crash-recovery helpers.
//!
//! Why this exists:
//! - copy/move flows can crash after filesystem work but before both databases
//!   reflect the final state
//! - the journal records enough metadata to reconcile source and target DB rows
//!   on the next startup
//!
//! Stage contract:
//! - `Intent`: journal row exists, no filesystem mutation is assumed yet
//! - `Staged`: data exists at `staged_relative` and has not been finalized
//! - `TargetDb`: the target-side DB update has committed
//! - `SourceDb`: the source-side DB cleanup has committed for moves

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::params;
use uuid::Uuid;

use super::util::{map_sql_error, normalize_relative_path, parse_relative_path_from_db};
use super::{Rating, SourceDatabase, SourceDbError};

/// File operation kinds tracked in the per-source journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOpKind {
    /// Moving a file between sources.
    Move,
    /// Copying a file into a source.
    Copy,
}

impl FileOpKind {
    fn as_str(self) -> &'static str {
        match self {
            FileOpKind::Move => "move",
            FileOpKind::Copy => "copy",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
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
    fn as_str(self) -> &'static str {
        match self {
            FileOpStage::Intent => "intent",
            FileOpStage::Staged => "staged",
            FileOpStage::TargetDb => "target_db",
            FileOpStage::SourceDb => "source_db",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
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

/// Summary of reconciliation work performed for pending file ops.
#[derive(Debug, Default)]
pub(crate) struct FileOpReconcileSummary {
    pub(crate) total: usize,
    pub(crate) completed: usize,
    pub(crate) errors: Vec<String>,
}

/// Result of loading journal rows, partitioned by valid and malformed entries.
#[derive(Debug, Default)]
pub(crate) struct ListedJournalEntries {
    pub(crate) entries: Vec<FileOpJournalEntry>,
    pub(crate) malformed: Vec<MalformedJournalEntry>,
}

/// Description of one malformed journal row that cannot be reconciled safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MalformedJournalEntry {
    pub(crate) id: Option<String>,
    pub(crate) detail: String,
}

impl MalformedJournalEntry {
    /// Build one malformed-row descriptor with optional row id context.
    fn new(id: Option<String>, detail: impl Into<String>) -> Self {
        Self {
            id,
            detail: detail.into(),
        }
    }

    /// Render one human-readable malformed-row error message for reconcile summaries.
    fn describe(&self) -> String {
        match self.id.as_deref() {
            Some(id) => format!("Malformed file-ops journal entry {id}: {}", self.detail),
            None => format!("Malformed file-ops journal entry: {}", self.detail),
        }
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

/// Insert a new journal entry before mutating the filesystem.
pub(crate) fn insert_entry(
    db: &SourceDatabase,
    entry: &FileOpJournalEntry,
) -> Result<(), SourceDbError> {
    let target_relative = normalize_relative_path(&entry.target_relative)?;
    let staged_relative = match entry.staged_relative.as_ref() {
        Some(path) => Some(normalize_relative_path(path)?),
        None => None,
    };
    let source_relative = match entry.source_relative.as_ref() {
        Some(path) => Some(normalize_relative_path(path)?),
        None => None,
    };
    db.connection
        .execute(
            "INSERT INTO file_ops_journal (
                id, op_type, stage, source_root, source_relative, target_relative,
                staged_relative, file_size, modified_ns, tag, looped, last_played_at, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                entry.id,
                entry.kind.as_str(),
                entry.stage.as_str(),
                entry
                    .source_root
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                source_relative,
                target_relative,
                staged_relative,
                entry.file_size.map(|size| size as i64),
                entry.modified_ns,
                entry.tag.map(|tag| tag.as_i64()),
                entry.looped.map(|looped| if looped { 1i64 } else { 0i64 }),
                entry.last_played_at,
                entry.created_at,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// Update a journal entry stage and optional metadata after filesystem work.
pub(crate) fn update_stage(
    db: &SourceDatabase,
    id: &str,
    stage: FileOpStage,
    file_size: Option<u64>,
    modified_ns: Option<i64>,
) -> Result<(), SourceDbError> {
    db.connection
        .execute(
            "UPDATE file_ops_journal
             SET stage = ?1,
                 file_size = COALESCE(?2, file_size),
                 modified_ns = COALESCE(?3, modified_ns)
             WHERE id = ?4",
            params![
                stage.as_str(),
                file_size.map(|size| size as i64),
                modified_ns,
                id,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// Remove a resolved journal entry after reconciliation.
pub(crate) fn remove_entry(db: &SourceDatabase, id: &str) -> Result<(), SourceDbError> {
    db.connection
        .execute("DELETE FROM file_ops_journal WHERE id = ?1", params![id])
        .map_err(map_sql_error)?;
    Ok(())
}

/// Load all pending journal entries for reconciliation.
pub(crate) fn list_entries(db: &SourceDatabase) -> Result<ListedJournalEntries, SourceDbError> {
    let mut stmt = db
        .connection
        .prepare(
            "SELECT id, op_type, stage, source_root, source_relative, target_relative,
                    staged_relative, file_size, modified_ns, tag, looped, last_played_at, created_at
             FROM file_ops_journal",
        )
        .map_err(map_sql_error)?;
    let mut rows = stmt.query([]).map_err(map_sql_error)?;
    let mut listed = ListedJournalEntries::default();
    while let Some(row) = rows.next().map_err(map_sql_error)? {
        match decode_journal_row(row) {
            Ok(entry) => listed.entries.push(entry),
            Err(malformed) => {
                tracing::warn!("{}", malformed.describe());
                listed.malformed.push(malformed);
            }
        }
    }
    Ok(listed)
}

/// Decode one persisted journal row into a typed recovery entry.
fn decode_journal_row(
    row: &rusqlite::Row<'_>,
) -> Result<FileOpJournalEntry, MalformedJournalEntry> {
    let id: String = row
        .get(0)
        .map_err(|err| malformed_column_error(None, "id", err))?;
    let op_type: String = row
        .get(1)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "op_type", err))?;
    let stage_text: String = row
        .get(2)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "stage", err))?;
    let kind = FileOpKind::from_str(op_type.as_str()).ok_or_else(|| {
        MalformedJournalEntry::new(
            Some(id.clone()),
            format!("unknown op_type value `{op_type}`"),
        )
    })?;
    let stage = FileOpStage::from_str(stage_text.as_str()).ok_or_else(|| {
        MalformedJournalEntry::new(
            Some(id.clone()),
            format!("unknown stage value `{stage_text}`"),
        )
    })?;
    let source_root: Option<String> = row
        .get(3)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "source_root", err))?;
    let source_relative_raw: Option<String> = row
        .get(4)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "source_relative", err))?;
    let target_relative_raw: String = row
        .get(5)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "target_relative", err))?;
    let staged_relative_raw: Option<String> = row
        .get(6)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "staged_relative", err))?;
    let file_size = row
        .get::<_, Option<i64>>(7)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "file_size", err))?
        .map(|size| {
            if size < 0 {
                Err(MalformedJournalEntry::new(
                    Some(id.clone()),
                    format!("file_size must be non-negative, got {size}"),
                ))
            } else {
                Ok(size as u64)
            }
        })
        .transpose()?;
    let modified_ns: Option<i64> = row
        .get(8)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "modified_ns", err))?;
    let tag = row
        .get::<_, Option<i64>>(9)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "tag", err))?
        .map(Rating::from_i64);
    let looped = row
        .get::<_, Option<i64>>(10)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "looped", err))?
        .map(|flag| flag != 0);
    let last_played_at: Option<i64> = row
        .get(11)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "last_played_at", err))?;
    let created_at: i64 = row
        .get(12)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "created_at", err))?;
    let source_relative =
        parse_optional_relative_path_column(id.as_str(), "source_relative", source_relative_raw)?;
    let target_relative =
        parse_required_relative_path_column(id.as_str(), "target_relative", target_relative_raw)?;
    let staged_relative =
        parse_optional_relative_path_column(id.as_str(), "staged_relative", staged_relative_raw)?;
    Ok(FileOpJournalEntry {
        id,
        kind,
        stage,
        source_root: source_root.map(PathBuf::from),
        source_relative,
        target_relative,
        staged_relative,
        file_size,
        modified_ns,
        tag,
        looped,
        last_played_at,
        created_at,
    })
}

/// Map one sqlite column read failure to a malformed-row descriptor.
fn malformed_column_error(
    id: Option<&str>,
    column: &str,
    err: rusqlite::Error,
) -> MalformedJournalEntry {
    let detail = format!("invalid `{column}` column: {err}");
    MalformedJournalEntry::new(id.map(str::to_string), detail)
}

/// Parse one optional relative-path column while preserving row-id context.
fn parse_optional_relative_path_column(
    id: &str,
    column: &str,
    value: Option<String>,
) -> Result<Option<PathBuf>, MalformedJournalEntry> {
    match value {
        Some(path) => parse_relative_path_from_db(&path).map(Some).map_err(|err| {
            MalformedJournalEntry::new(
                Some(id.to_string()),
                format!("invalid `{column}` path `{path}`: {err}"),
            )
        }),
        None => Ok(None),
    }
}

/// Parse one required relative-path column while preserving row-id context.
fn parse_required_relative_path_column(
    id: &str,
    column: &str,
    value: String,
) -> Result<PathBuf, MalformedJournalEntry> {
    parse_relative_path_from_db(&value).map_err(|err| {
        MalformedJournalEntry::new(
            Some(id.to_string()),
            format!("invalid `{column}` path `{value}`: {err}"),
        )
    })
}

/// Reconcile all pending file ops against the filesystem and database.
pub(crate) fn reconcile_pending_ops(db: &SourceDatabase) -> Result<FileOpReconcileSummary, String> {
    let listed = list_entries(db).map_err(|err| err.to_string())?;
    let mut summary = FileOpReconcileSummary {
        total: listed.entries.len() + listed.malformed.len(),
        completed: 0,
        errors: Vec::new(),
    };
    for malformed in listed.malformed {
        let message = malformed.describe();
        if let Some(id) = malformed.id.as_deref() {
            match remove_entry(db, id) {
                Ok(()) => summary
                    .errors
                    .push(format!("{message}; dropped malformed journal row")),
                Err(err) => summary.errors.push(format!(
                    "{message}; failed to drop malformed row {id}: {err}"
                )),
            }
        } else {
            summary.errors.push(message);
        }
    }
    for entry in listed.entries {
        match reconcile_entry(db, &entry) {
            Ok(()) => {
                if let Err(err) = remove_entry(db, &entry.id) {
                    summary.errors.push(format!(
                        "Failed to remove journal entry {}: {err}",
                        entry.id
                    ));
                } else {
                    summary.completed += 1;
                }
            }
            Err(err) => summary.errors.push(err),
        }
    }
    Ok(summary)
}

fn reconcile_entry(db: &SourceDatabase, entry: &FileOpJournalEntry) -> Result<(), String> {
    let target_root = db.root();
    let target_absolute = target_root.join(&entry.target_relative);
    let staged_absolute = entry
        .staged_relative
        .as_ref()
        .map(|path| target_root.join(path));
    reconcile_staged_file(staged_absolute.as_deref(), &target_absolute)?;
    let target_exists = reconcile_target_entry(db, entry, &target_absolute)?;
    if entry.kind == FileOpKind::Move {
        reconcile_source_entry(db, entry, target_exists)?;
    }
    Ok(())
}

/// Finalize one staged file into the target path or clean the stale staged copy.
fn reconcile_staged_file(
    staged_absolute: Option<&Path>,
    target_absolute: &Path,
) -> Result<(), String> {
    let Some(staged_absolute) = staged_absolute else {
        return Ok(());
    };
    if !staged_absolute.is_file() {
        return Ok(());
    }
    if !target_absolute.is_file() {
        if let Some(parent) = target_absolute.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create target dir: {err}"))?;
        }
        std::fs::rename(staged_absolute, target_absolute)
            .map_err(|err| format!("Failed to finalize staged file: {err}"))?;
    } else {
        std::fs::remove_file(staged_absolute)
            .map_err(|err| format!("Failed to remove staged file: {err}"))?;
    }
    Ok(())
}

/// Reconcile one target DB row and return whether the target file exists afterwards.
fn reconcile_target_entry(
    db: &SourceDatabase,
    entry: &FileOpJournalEntry,
    target_absolute: &Path,
) -> Result<bool, String> {
    if target_absolute.is_file() {
        let (file_size, modified_ns) = file_metadata(target_absolute)?;
        let mut batch = db.write_batch().map_err(|err| err.to_string())?;
        batch
            .upsert_file(&entry.target_relative, file_size, modified_ns)
            .map_err(|err| err.to_string())?;
        if let Some(tag) = entry.tag {
            batch
                .set_tag(&entry.target_relative, tag)
                .map_err(|err| err.to_string())?;
        }
        if let Some(looped) = entry.looped {
            batch
                .set_looped(&entry.target_relative, looped)
                .map_err(|err| err.to_string())?;
        }
        if let Some(last_played_at) = entry.last_played_at {
            batch
                .set_last_played_at(&entry.target_relative, last_played_at)
                .map_err(|err| err.to_string())?;
        }
        batch.commit().map_err(|err| err.to_string())?;
        Ok(true)
    } else {
        db.remove_file(&entry.target_relative)
            .map_err(|err| format!("Failed to drop target DB row: {err}"))?;
        Ok(false)
    }
}

fn reconcile_source_entry(
    target_db: &SourceDatabase,
    entry: &FileOpJournalEntry,
    target_exists: bool,
) -> Result<(), String> {
    let Some(source_root) = entry.source_root.as_ref() else {
        return Ok(());
    };
    let Some(source_relative) = entry.source_relative.as_ref() else {
        return Ok(());
    };
    if !source_root.is_dir() {
        return Ok(());
    }
    let source_absolute = source_root.join(source_relative);
    if source_absolute.is_file() && !target_exists {
        return Ok(());
    }
    let source_db = SourceDatabase::open(source_root)
        .map_err(|err| format!("Failed to open source DB for recovery: {err}"))?;
    if !source_absolute.is_file() {
        source_db
            .remove_file(source_relative)
            .map_err(|err| format!("Failed to drop source DB row: {err}"))?;
    } else if target_exists {
        tracing::warn!(
            "Move recovery left duplicate file at {} -> {}",
            source_absolute.display(),
            target_db.root().display()
        );
    }
    Ok(())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "File modified time is before epoch".to_string())?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}

fn now_epoch_seconds() -> Result<i64, SourceDbError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SourceDbError::Unexpected)?;
    Ok(now.as_secs() as i64)
}

/// Behavior and recovery-matrix tests for the journal lifecycle.
#[cfg(test)]
mod tests;
