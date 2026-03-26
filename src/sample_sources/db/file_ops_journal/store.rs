use std::path::PathBuf;

use rusqlite::params;

use super::super::util::{map_sql_error, normalize_relative_path, parse_relative_path_from_db};
use super::super::{Rating, SourceDatabase, SourceDbError};
use super::entry::{FileOpJournalEntry, FileOpKind, FileOpStage};

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
    pub(super) fn new(id: Option<String>, detail: impl Into<String>) -> Self {
        Self {
            id,
            detail: detail.into(),
        }
    }

    /// Render one human-readable malformed-row error message for reconcile summaries.
    pub(super) fn describe(&self) -> String {
        match self.id.as_deref() {
            Some(id) => format!("Malformed file-ops journal entry {id}: {}", self.detail),
            None => format!("Malformed file-ops journal entry: {}", self.detail),
        }
    }
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
                 staged_relative, file_size, modified_ns, tag, looped, locked, last_played_at,
                 created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
                entry.locked.map(|locked| if locked { 1i64 } else { 0i64 }),
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
                    staged_relative, file_size, modified_ns, tag, looped, locked, last_played_at,
                    created_at
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
    let locked = row
        .get::<_, Option<i64>>(11)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "locked", err))?
        .map(|flag| flag != 0);
    let last_played_at: Option<i64> = row
        .get(12)
        .map_err(|err| malformed_column_error(Some(id.as_str()), "last_played_at", err))?;
    let created_at: i64 = row
        .get(13)
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
        locked,
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
