use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use super::LibraryError;
use super::connection::LibraryDatabase;
use super::error::map_sql_error;
use crate::sample_sources::SourceId;

/// Workflow state for an origin file in a harvest queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarvestState {
    /// Discovered but not opened or processed.
    New,
    /// Loaded, auditioned, or focused.
    Seen,
    /// Rated, tagged, marked, edited, copied, or otherwise acted on.
    Touched,
    /// User explicitly finished harvesting this file.
    Done,
    /// User explicitly hid this file from harvest queues.
    Ignored,
}

impl HarvestState {
    /// Stable database representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Seen => "seen",
            Self::Touched => "touched",
            Self::Done => "done",
            Self::Ignored => "ignored",
        }
    }

    /// Parse stored state, defaulting unknown values to `New`.
    pub fn from_stored(value: &str) -> Self {
        match value {
            "seen" => Self::Seen,
            "touched" => Self::Touched,
            "done" => Self::Done,
            "ignored" => Self::Ignored,
            _ => Self::New,
        }
    }

    fn advance_automatically(self, requested: Self) -> Self {
        match self {
            Self::Done | Self::Ignored | Self::Touched => self,
            Self::Seen => match requested {
                Self::Touched => Self::Touched,
                _ => Self::Seen,
            },
            Self::New => match requested {
                Self::Seen | Self::Touched => requested,
                _ => Self::New,
            },
        }
    }
}

/// Source-scoped identity used to track a file even when graph edges cross sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestFileKey {
    /// Stable source id from the global source catalog.
    pub source_id: SourceId,
    /// Path relative to the source root.
    pub relative_path: PathBuf,
}

impl HarvestFileKey {
    /// Build a harvest key.
    pub fn new(source_id: SourceId, relative_path: PathBuf) -> Self {
        Self {
            source_id,
            relative_path,
        }
    }
}

/// Current durable identity hints for a harvest-tracked file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestFileIdentity {
    /// Source-scoped file key.
    pub key: HarvestFileKey,
    /// File size in bytes when known.
    pub file_size: Option<u64>,
    /// Last modified time in nanoseconds when known.
    pub modified_ns: Option<i64>,
    /// Content hash when available from source metadata or analysis cache.
    pub content_hash: Option<String>,
}

impl HarvestFileIdentity {
    /// Build a harvest identity.
    pub fn new(key: HarvestFileKey) -> Self {
        Self {
            key,
            file_size: None,
            modified_ns: None,
            content_hash: None,
        }
    }
}

/// Stored harvest row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarvestFileRecord {
    /// Source-scoped file key.
    pub key: HarvestFileKey,
    /// File size in bytes when known.
    pub file_size: Option<u64>,
    /// Last modified time in nanoseconds when known.
    pub modified_ns: Option<i64>,
    /// Content hash when available.
    pub content_hash: Option<String>,
    /// Workflow state.
    pub state: HarvestState,
    /// First discovery timestamp.
    pub discovered_at: i64,
    /// First seen timestamp.
    pub seen_at: Option<i64>,
    /// First touched timestamp.
    pub touched_at: Option<i64>,
    /// Manual done timestamp.
    pub done_at: Option<i64>,
    /// Manual ignored timestamp.
    pub ignored_at: Option<i64>,
    /// Optional user note reserved for later UI.
    pub note: Option<String>,
}

/// Operation that created a derived file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarvestDerivationOperation {
    /// Direct file copy into another harvest-visible location.
    Copy,
    /// Selection extraction or chop render.
    Extract,
    /// Crop rendered to a copy.
    CropCopy,
    /// Trim rendered to a copy.
    TrimCopy,
    /// Reverse rendered to a copy.
    ReverseCopy,
    /// Edit effects rendered to a copy.
    EditCopy,
    /// Normalize rendered to a copy.
    NormalizeCopy,
    /// Export or handoff render.
    Export,
    /// Copy into the primary library.
    CopyToPrimary,
    /// Forward-compatible operation string.
    Other(String),
}

impl HarvestDerivationOperation {
    /// Stable database representation.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Copy => "copy",
            Self::Extract => "extract",
            Self::CropCopy => "crop_copy",
            Self::TrimCopy => "trim_copy",
            Self::ReverseCopy => "reverse_copy",
            Self::EditCopy => "edit_copy",
            Self::NormalizeCopy => "normalize_copy",
            Self::Export => "export",
            Self::CopyToPrimary => "copy_to_primary",
            Self::Other(value) => value.as_str(),
        }
    }

    /// Parse a stored operation while preserving unknown operation names.
    pub fn from_stored(value: String) -> Self {
        match value.as_str() {
            "copy" => Self::Copy,
            "extract" => Self::Extract,
            "crop_copy" => Self::CropCopy,
            "trim_copy" => Self::TrimCopy,
            "reverse_copy" => Self::ReverseCopy,
            "edit_copy" => Self::EditCopy,
            "normalize_copy" => Self::NormalizeCopy,
            "export" => Self::Export,
            "copy_to_primary" => Self::CopyToPrimary,
            _ => Self::Other(value),
        }
    }
}

/// Source range captured by a derivation edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarvestSourceRange {
    /// Start in seconds.
    pub start_seconds: f64,
    /// End in seconds.
    pub end_seconds: f64,
}

/// Metadata copied from an origin at the moment a derivative was created.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarvestMetadataSnapshot {
    /// Rating value when known.
    pub rating: Option<i64>,
    /// Tags present on the origin when the derivative was created.
    pub tags: Vec<String>,
    /// Playback type such as `one-shot` or `loop`.
    pub playback_type: Option<String>,
}

/// New graph edge to record.
#[derive(Debug, Clone, PartialEq)]
pub struct NewHarvestDerivation {
    /// Origin identity.
    pub parent: HarvestFileIdentity,
    /// Derived file identity.
    pub child: HarvestFileIdentity,
    /// Creation operation.
    pub operation: HarvestDerivationOperation,
    /// Optional source range used by the operation.
    pub source_range: Option<HarvestSourceRange>,
    /// Optional output duration in seconds.
    pub output_duration_seconds: Option<f64>,
    /// Destination folder used for the derived file.
    pub destination_folder: Option<PathBuf>,
    /// Metadata inherited from the parent at creation time.
    pub inherited_metadata: HarvestMetadataSnapshot,
    /// Tool/app version used to create the edge.
    pub tool_version: String,
}

/// Stored graph edge.
#[derive(Debug, Clone, PartialEq)]
pub struct HarvestDerivationRecord {
    /// Database id.
    pub id: i64,
    /// Origin identity.
    pub parent: HarvestFileIdentity,
    /// Derived file identity.
    pub child: HarvestFileIdentity,
    /// Creation operation.
    pub operation: HarvestDerivationOperation,
    /// Optional source range used by the operation.
    pub source_range: Option<HarvestSourceRange>,
    /// Optional output duration in seconds.
    pub output_duration_seconds: Option<f64>,
    /// Destination folder used for the derived file.
    pub destination_folder: Option<PathBuf>,
    /// Metadata inherited from the parent at creation time.
    pub inherited_metadata: HarvestMetadataSnapshot,
    /// Tool/app version used to create the edge.
    pub tool_version: String,
    /// Edge creation timestamp.
    pub created_at: i64,
}

impl LibraryDatabase {
    pub(super) fn upsert_harvest_file(
        &self,
        identity: &HarvestFileIdentity,
    ) -> Result<HarvestFileRecord, LibraryError> {
        let now = now_unix_seconds();
        upsert_harvest_file_on_connection(&self.connection, identity, now)?;
        self.harvest_file(&identity.key)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows.into())
    }

    pub(super) fn advance_harvest_state(
        &self,
        identity: &HarvestFileIdentity,
        requested: HarvestState,
    ) -> Result<HarvestFileRecord, LibraryError> {
        let now = now_unix_seconds();
        upsert_harvest_file_on_connection(&self.connection, identity, now)?;
        let Some(record) = self.harvest_file(&identity.key)? else {
            return Err(rusqlite::Error::QueryReturnedNoRows.into());
        };
        let next = record.state.advance_automatically(requested);
        write_harvest_state_on_connection(&self.connection, &identity.key, next, now, false)?;
        self.harvest_file(&identity.key)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows.into())
    }

    pub(super) fn set_harvest_state(
        &self,
        key: &HarvestFileKey,
        state: HarvestState,
    ) -> Result<HarvestFileRecord, LibraryError> {
        let now = now_unix_seconds();
        let identity = HarvestFileIdentity::new(key.clone());
        upsert_harvest_file_on_connection(&self.connection, &identity, now)?;
        write_harvest_state_on_connection(&self.connection, key, state, now, true)?;
        self.harvest_file(key)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows.into())
    }

    pub(super) fn harvest_file(
        &self,
        key: &HarvestFileKey,
    ) -> Result<Option<HarvestFileRecord>, LibraryError> {
        self.connection
            .query_row(
                "SELECT source_id, relative_path, file_size, modified_ns, content_hash,
                    harvest_state, discovered_at, seen_at, touched_at, done_at, ignored_at, note
                 FROM harvest_files
                 WHERE source_id = ?1 AND relative_path = ?2",
                params![key.source_id.as_str(), stored_path(&key.relative_path)],
                harvest_file_from_row,
            )
            .optional()
            .map_err(map_sql_error)
    }

    pub(super) fn record_harvest_derivation(
        &mut self,
        edge: &NewHarvestDerivation,
    ) -> Result<i64, LibraryError> {
        let now = now_unix_seconds();
        let tx = self.connection.transaction().map_err(map_sql_error)?;
        upsert_harvest_file_on_transaction(&tx, &edge.parent, now)?;
        upsert_harvest_file_on_transaction(&tx, &edge.child, now)?;
        write_harvest_state_on_transaction(
            &tx,
            &edge.parent.key,
            HarvestState::Touched,
            now,
            false,
        )?;
        Self::insert_harvest_derivation_in(&tx, edge, now)?;
        let id = tx.last_insert_rowid();
        tx.commit().map_err(map_sql_error)?;
        Ok(id)
    }

    pub(super) fn harvest_derivations_for_parent(
        &self,
        key: &HarvestFileKey,
    ) -> Result<Vec<HarvestDerivationRecord>, LibraryError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id,
                    parent_source_id, parent_relative_path, parent_file_size,
                    parent_modified_ns, parent_content_hash,
                    child_source_id, child_relative_path, child_file_size,
                    child_modified_ns, child_content_hash,
                    operation, source_range_start, source_range_end,
                    output_duration_seconds, destination_folder,
                    inherited_rating, inherited_tags_json, inherited_playback_type,
                    tool_version, created_at
                 FROM harvest_derivations
                 WHERE parent_source_id = ?1 AND parent_relative_path = ?2
                 ORDER BY created_at ASC, id ASC",
            )
            .map_err(map_sql_error)?;
        stmt.query_map(
            params![key.source_id.as_str(), stored_path(&key.relative_path)],
            harvest_derivation_from_row,
        )
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)
    }

    pub(super) fn harvest_parents_for_child(
        &self,
        key: &HarvestFileKey,
    ) -> Result<Vec<HarvestDerivationRecord>, LibraryError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT id,
                    parent_source_id, parent_relative_path, parent_file_size,
                    parent_modified_ns, parent_content_hash,
                    child_source_id, child_relative_path, child_file_size,
                    child_modified_ns, child_content_hash,
                    operation, source_range_start, source_range_end,
                    output_duration_seconds, destination_folder,
                    inherited_rating, inherited_tags_json, inherited_playback_type,
                    tool_version, created_at
                 FROM harvest_derivations
                 WHERE child_source_id = ?1 AND child_relative_path = ?2
                 ORDER BY created_at ASC, id ASC",
            )
            .map_err(map_sql_error)?;
        stmt.query_map(
            params![key.source_id.as_str(), stored_path(&key.relative_path)],
            harvest_derivation_from_row,
        )
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)
    }

    pub(super) fn harvest_derivative_count(
        &self,
        key: &HarvestFileKey,
    ) -> Result<u64, LibraryError> {
        let count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*)
                 FROM harvest_derivations
                 WHERE parent_source_id = ?1 AND parent_relative_path = ?2",
                params![key.source_id.as_str(), stored_path(&key.relative_path)],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        Ok(count.max(0) as u64)
    }

    pub(super) fn harvest_files_for_source(
        &self,
        source_id: &SourceId,
    ) -> Result<Vec<HarvestFileRecord>, LibraryError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT source_id, relative_path, file_size, modified_ns, content_hash,
                    harvest_state, discovered_at, seen_at, touched_at, done_at, ignored_at, note
                 FROM harvest_files
                 WHERE source_id = ?1
                 ORDER BY relative_path ASC",
            )
            .map_err(map_sql_error)?;
        stmt.query_map([source_id.as_str()], harvest_file_from_row)
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)
    }

    pub(super) fn harvest_derivative_counts_for_source(
        &self,
        source_id: &SourceId,
    ) -> Result<Vec<(PathBuf, u64)>, LibraryError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT parent_relative_path, COUNT(*) AS derivative_count
                 FROM harvest_derivations
                 WHERE parent_source_id = ?1
                 GROUP BY parent_relative_path
                 ORDER BY parent_relative_path ASC",
            )
            .map_err(map_sql_error)?;
        stmt.query_map([source_id.as_str()], |row| {
            let relative_path: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((PathBuf::from(relative_path), count.max(0) as u64))
        })
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)
    }

    pub(super) fn remap_harvest_file_key(
        &mut self,
        old_key: &HarvestFileKey,
        new_key: &HarvestFileKey,
    ) -> Result<usize, LibraryError> {
        let tx = self.connection.transaction().map_err(map_sql_error)?;
        let changed = remap_harvest_file_key_on_transaction(&tx, old_key, new_key)?;
        tx.commit().map_err(map_sql_error)?;
        Ok(changed)
    }

    pub(super) fn remap_harvest_file_prefix(
        &mut self,
        source_id: &SourceId,
        old_prefix: &Path,
        new_prefix: &Path,
    ) -> Result<usize, LibraryError> {
        if old_prefix == new_prefix || old_prefix.as_os_str().is_empty() {
            return Ok(0);
        }
        let mappings = self.harvest_key_prefix_mappings(source_id, old_prefix, new_prefix)?;
        if mappings.is_empty() {
            return Ok(0);
        }
        let tx = self.connection.transaction().map_err(map_sql_error)?;
        let mut changed = 0;
        for (old_key, new_key) in mappings {
            changed += remap_harvest_file_key_on_transaction(&tx, &old_key, &new_key)?;
        }
        tx.commit().map_err(map_sql_error)?;
        Ok(changed)
    }

    fn harvest_key_prefix_mappings(
        &self,
        source_id: &SourceId,
        old_prefix: &Path,
        new_prefix: &Path,
    ) -> Result<Vec<(HarvestFileKey, HarvestFileKey)>, LibraryError> {
        let mappings = harvest_related_paths_for_source(&self.connection, source_id)?
            .into_iter()
            .filter_map(|relative_path| {
                remap_relative_path_prefix(&relative_path, old_prefix, new_prefix).map(
                    |new_relative_path| {
                        (
                            HarvestFileKey::new(source_id.clone(), relative_path),
                            HarvestFileKey::new(source_id.clone(), new_relative_path),
                        )
                    },
                )
            })
            .filter(|(old_key, new_key)| old_key != new_key)
            .collect::<Vec<_>>();
        Ok(mappings)
    }

    fn insert_harvest_derivation_in(
        tx: &Transaction<'_>,
        edge: &NewHarvestDerivation,
        now: i64,
    ) -> Result<(), LibraryError> {
        let inherited_tags_json = serde_json::to_string(&edge.inherited_metadata.tags)?;
        let source_start = edge.source_range.map(|range| range.start_seconds);
        let source_end = edge.source_range.map(|range| range.end_seconds);
        tx.execute(
            "INSERT INTO harvest_derivations (
                parent_source_id, parent_relative_path, parent_file_size,
                parent_modified_ns, parent_content_hash,
                child_source_id, child_relative_path, child_file_size,
                child_modified_ns, child_content_hash,
                operation, source_range_start, source_range_end,
                output_duration_seconds, destination_folder,
                inherited_rating, inherited_tags_json, inherited_playback_type,
                tool_version, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20
             )",
            params![
                edge.parent.key.source_id.as_str(),
                stored_path(&edge.parent.key.relative_path),
                optional_u64_to_i64(edge.parent.file_size),
                edge.parent.modified_ns,
                edge.parent.content_hash.as_deref(),
                edge.child.key.source_id.as_str(),
                stored_path(&edge.child.key.relative_path),
                optional_u64_to_i64(edge.child.file_size),
                edge.child.modified_ns,
                edge.child.content_hash.as_deref(),
                edge.operation.as_str(),
                source_start,
                source_end,
                edge.output_duration_seconds,
                edge.destination_folder
                    .as_ref()
                    .map(|path| stored_path(path)),
                edge.inherited_metadata.rating,
                inherited_tags_json,
                edge.inherited_metadata.playback_type.as_deref(),
                edge.tool_version,
                now,
            ],
        )
        .map_err(map_sql_error)?;
        Ok(())
    }
}

fn remap_harvest_file_key_on_transaction(
    tx: &Transaction<'_>,
    old_key: &HarvestFileKey,
    new_key: &HarvestFileKey,
) -> Result<usize, LibraryError> {
    if old_key == new_key {
        return Ok(0);
    }
    let old_record = harvest_file_on_transaction(tx, old_key)?;
    let mut changed = 0;
    if let Some(record) = old_record {
        changed += merge_harvest_file_record_on_transaction(tx, new_key, &record)?;
        changed += tx
            .execute(
                "DELETE FROM harvest_files
                 WHERE source_id = ?1 AND relative_path = ?2",
                params![
                    old_key.source_id.as_str(),
                    stored_path(&old_key.relative_path),
                ],
            )
            .map_err(map_sql_error)?;
    }
    changed += tx
        .execute(
            "UPDATE harvest_derivations
             SET parent_source_id = ?3,
                 parent_relative_path = ?4
             WHERE parent_source_id = ?1 AND parent_relative_path = ?2",
            params![
                old_key.source_id.as_str(),
                stored_path(&old_key.relative_path),
                new_key.source_id.as_str(),
                stored_path(&new_key.relative_path),
            ],
        )
        .map_err(map_sql_error)?;
    changed += tx
        .execute(
            "UPDATE harvest_derivations
             SET child_source_id = ?3,
                 child_relative_path = ?4
             WHERE child_source_id = ?1 AND child_relative_path = ?2",
            params![
                old_key.source_id.as_str(),
                stored_path(&old_key.relative_path),
                new_key.source_id.as_str(),
                stored_path(&new_key.relative_path),
            ],
        )
        .map_err(map_sql_error)?;
    Ok(changed)
}

fn harvest_file_on_transaction(
    tx: &Transaction<'_>,
    key: &HarvestFileKey,
) -> Result<Option<HarvestFileRecord>, LibraryError> {
    tx.query_row(
        "SELECT source_id, relative_path, file_size, modified_ns, content_hash,
            harvest_state, discovered_at, seen_at, touched_at, done_at, ignored_at, note
         FROM harvest_files
         WHERE source_id = ?1 AND relative_path = ?2",
        params![key.source_id.as_str(), stored_path(&key.relative_path)],
        harvest_file_from_row,
    )
    .optional()
    .map_err(map_sql_error)
}

fn harvest_related_paths_for_source(
    connection: &Connection,
    source_id: &SourceId,
) -> Result<Vec<PathBuf>, LibraryError> {
    let mut paths = BTreeSet::new();
    collect_harvest_paths_for_source(
        connection,
        "SELECT relative_path FROM harvest_files WHERE source_id = ?1",
        source_id,
        &mut paths,
    )?;
    collect_harvest_paths_for_source(
        connection,
        "SELECT DISTINCT parent_relative_path
         FROM harvest_derivations
         WHERE parent_source_id = ?1",
        source_id,
        &mut paths,
    )?;
    collect_harvest_paths_for_source(
        connection,
        "SELECT DISTINCT child_relative_path
         FROM harvest_derivations
         WHERE child_source_id = ?1",
        source_id,
        &mut paths,
    )?;
    Ok(paths.into_iter().collect())
}

fn collect_harvest_paths_for_source(
    connection: &Connection,
    sql: &str,
    source_id: &SourceId,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), LibraryError> {
    let mut stmt = connection.prepare(sql).map_err(map_sql_error)?;
    for path in stmt
        .query_map([source_id.as_str()], |row| row.get::<_, String>(0))
        .map_err(map_sql_error)?
    {
        paths.insert(PathBuf::from(path.map_err(map_sql_error)?));
    }
    Ok(())
}

fn remap_relative_path_prefix(
    relative_path: &Path,
    old_prefix: &Path,
    new_prefix: &Path,
) -> Option<PathBuf> {
    let suffix = relative_path.strip_prefix(old_prefix).ok()?;
    Some(new_prefix.join(suffix))
}

fn merge_harvest_file_record_on_transaction(
    tx: &Transaction<'_>,
    key: &HarvestFileKey,
    record: &HarvestFileRecord,
) -> Result<usize, LibraryError> {
    tx.execute(
        "INSERT INTO harvest_files (
            source_id, relative_path, file_size, modified_ns, content_hash,
            harvest_state, discovered_at, seen_at, touched_at, done_at, ignored_at, note
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(source_id, relative_path) DO UPDATE SET
            file_size = COALESCE(excluded.file_size, harvest_files.file_size),
            modified_ns = COALESCE(excluded.modified_ns, harvest_files.modified_ns),
            content_hash = COALESCE(excluded.content_hash, harvest_files.content_hash),
            harvest_state = CASE
                WHEN harvest_files.harvest_state IN ('done', 'ignored') THEN harvest_files.harvest_state
                WHEN excluded.harvest_state IN ('done', 'ignored') THEN excluded.harvest_state
                WHEN harvest_files.harvest_state = 'touched' OR excluded.harvest_state = 'touched' THEN 'touched'
                WHEN harvest_files.harvest_state = 'seen' OR excluded.harvest_state = 'seen' THEN 'seen'
                ELSE 'new'
            END,
            discovered_at = MIN(harvest_files.discovered_at, excluded.discovered_at),
            seen_at = COALESCE(harvest_files.seen_at, excluded.seen_at),
            touched_at = COALESCE(harvest_files.touched_at, excluded.touched_at),
            done_at = COALESCE(harvest_files.done_at, excluded.done_at),
            ignored_at = COALESCE(harvest_files.ignored_at, excluded.ignored_at),
            note = COALESCE(harvest_files.note, excluded.note)",
        params![
            key.source_id.as_str(),
            stored_path(&key.relative_path),
            optional_u64_to_i64(record.file_size),
            record.modified_ns,
            record.content_hash.as_deref(),
            record.state.as_str(),
            record.discovered_at,
            record.seen_at,
            record.touched_at,
            record.done_at,
            record.ignored_at,
            record.note.as_deref(),
        ],
    )
    .map_err(map_sql_error)
}

fn upsert_harvest_file_on_connection(
    connection: &Connection,
    identity: &HarvestFileIdentity,
    now: i64,
) -> Result<(), LibraryError> {
    connection
        .execute(
            "INSERT INTO harvest_files (
                source_id, relative_path, file_size, modified_ns, content_hash,
                harvest_state, discovered_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'new', ?6)
             ON CONFLICT(source_id, relative_path) DO UPDATE SET
                file_size = COALESCE(excluded.file_size, harvest_files.file_size),
                modified_ns = COALESCE(excluded.modified_ns, harvest_files.modified_ns),
                content_hash = COALESCE(excluded.content_hash, harvest_files.content_hash)",
            params![
                identity.key.source_id.as_str(),
                stored_path(&identity.key.relative_path),
                optional_u64_to_i64(identity.file_size),
                identity.modified_ns,
                identity.content_hash.as_deref(),
                now,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn upsert_harvest_file_on_transaction(
    tx: &Transaction<'_>,
    identity: &HarvestFileIdentity,
    now: i64,
) -> Result<(), LibraryError> {
    tx.execute(
        "INSERT INTO harvest_files (
            source_id, relative_path, file_size, modified_ns, content_hash,
            harvest_state, discovered_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'new', ?6)
         ON CONFLICT(source_id, relative_path) DO UPDATE SET
            file_size = COALESCE(excluded.file_size, harvest_files.file_size),
            modified_ns = COALESCE(excluded.modified_ns, harvest_files.modified_ns),
            content_hash = COALESCE(excluded.content_hash, harvest_files.content_hash)",
        params![
            identity.key.source_id.as_str(),
            stored_path(&identity.key.relative_path),
            optional_u64_to_i64(identity.file_size),
            identity.modified_ns,
            identity.content_hash.as_deref(),
            now,
        ],
    )
    .map_err(map_sql_error)?;
    Ok(())
}

fn write_harvest_state_on_connection(
    connection: &Connection,
    key: &HarvestFileKey,
    state: HarvestState,
    now: i64,
    manual: bool,
) -> Result<(), LibraryError> {
    let reset = manual && state == HarvestState::New;
    connection
        .execute(
            harvest_state_update_sql(),
            params![
                state.as_str(),
                key.source_id.as_str(),
                stored_path(&key.relative_path),
                now,
                reset,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn write_harvest_state_on_transaction(
    tx: &Transaction<'_>,
    key: &HarvestFileKey,
    state: HarvestState,
    now: i64,
    manual: bool,
) -> Result<(), LibraryError> {
    let reset = manual && state == HarvestState::New;
    tx.execute(
        harvest_state_update_sql(),
        params![
            state.as_str(),
            key.source_id.as_str(),
            stored_path(&key.relative_path),
            now,
            reset,
        ],
    )
    .map_err(map_sql_error)?;
    Ok(())
}

fn harvest_state_update_sql() -> &'static str {
    "UPDATE harvest_files
     SET harvest_state = ?1,
         seen_at = CASE
            WHEN ?5 THEN NULL
            WHEN ?1 IN ('seen', 'touched') THEN COALESCE(seen_at, ?4)
            ELSE seen_at
         END,
         touched_at = CASE
            WHEN ?5 THEN NULL
            WHEN ?1 = 'touched' THEN COALESCE(touched_at, ?4)
            ELSE touched_at
         END,
         done_at = CASE
            WHEN ?5 THEN NULL
            WHEN ?1 = 'done' THEN COALESCE(done_at, ?4)
            ELSE done_at
         END,
         ignored_at = CASE
            WHEN ?5 THEN NULL
            WHEN ?1 = 'ignored' THEN COALESCE(ignored_at, ?4)
            ELSE ignored_at
         END
     WHERE source_id = ?2 AND relative_path = ?3"
}

fn harvest_file_from_row(row: &Row<'_>) -> rusqlite::Result<HarvestFileRecord> {
    let source_id: String = row.get(0)?;
    let relative_path: String = row.get(1)?;
    let file_size: Option<i64> = row.get(2)?;
    let state: String = row.get(5)?;
    Ok(HarvestFileRecord {
        key: HarvestFileKey::new(
            SourceId::from_string(source_id),
            PathBuf::from(relative_path),
        ),
        file_size: file_size.and_then(nonnegative_i64_to_u64),
        modified_ns: row.get(3)?,
        content_hash: row.get(4)?,
        state: HarvestState::from_stored(&state),
        discovered_at: row.get(6)?,
        seen_at: row.get(7)?,
        touched_at: row.get(8)?,
        done_at: row.get(9)?,
        ignored_at: row.get(10)?,
        note: row.get(11)?,
    })
}

fn harvest_derivation_from_row(row: &Row<'_>) -> rusqlite::Result<HarvestDerivationRecord> {
    let operation: String = row.get(11)?;
    let inherited_tags_json: String = row.get(17)?;
    let tags = serde_json::from_str::<Vec<String>>(&inherited_tags_json).map_err(|source| {
        rusqlite::Error::FromSqlConversionFailure(17, Type::Text, Box::new(source))
    })?;
    let source_start: Option<f64> = row.get(12)?;
    let source_end: Option<f64> = row.get(13)?;
    let source_range = match (source_start, source_end) {
        (Some(start_seconds), Some(end_seconds)) => Some(HarvestSourceRange {
            start_seconds,
            end_seconds,
        }),
        _ => None,
    };
    Ok(HarvestDerivationRecord {
        id: row.get(0)?,
        parent: HarvestFileIdentity {
            key: HarvestFileKey::new(
                SourceId::from_string(row.get::<_, String>(1)?),
                PathBuf::from(row.get::<_, String>(2)?),
            ),
            file_size: row
                .get::<_, Option<i64>>(3)?
                .and_then(nonnegative_i64_to_u64),
            modified_ns: row.get(4)?,
            content_hash: row.get(5)?,
        },
        child: HarvestFileIdentity {
            key: HarvestFileKey::new(
                SourceId::from_string(row.get::<_, String>(6)?),
                PathBuf::from(row.get::<_, String>(7)?),
            ),
            file_size: row
                .get::<_, Option<i64>>(8)?
                .and_then(nonnegative_i64_to_u64),
            modified_ns: row.get(9)?,
            content_hash: row.get(10)?,
        },
        operation: HarvestDerivationOperation::from_stored(operation),
        source_range,
        output_duration_seconds: row.get(14)?,
        destination_folder: row.get::<_, Option<String>>(15)?.map(PathBuf::from),
        inherited_metadata: HarvestMetadataSnapshot {
            rating: row.get(16)?,
            tags,
            playback_type: row.get(18)?,
        },
        tool_version: row.get(19)?,
        created_at: row.get(20)?,
    })
}

fn stored_path(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

fn optional_u64_to_i64(value: Option<u64>) -> Option<i64> {
    value.map(|value| i64::try_from(value).unwrap_or(i64::MAX))
}

fn nonnegative_i64_to_u64(value: i64) -> Option<u64> {
    u64::try_from(value).ok()
}

fn now_unix_seconds() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}
