use std::path::Path;
use std::time::Instant;

use rusqlite::{OptionalExtension, Transaction, params};

use super::super::util::{map_sql_error, normalize_relative_path};
use super::super::{SourceDatabase, SourceDbError, SourceTag, SourceWriteBatch};
use super::identity::normalize_tag_identity;
use crate::diagnostics::{DbDebugEvent, emit_db_debug_event};

impl SourceDatabase {
    /// Create or resolve a normal tag and assign it to one wav path in a single transaction.
    pub fn assign_tag_to_path(
        &self,
        relative_path: &Path,
        label: &str,
    ) -> Result<SourceTag, SourceDbError> {
        let started_at = Instant::now();
        let mut batch = self.write_batch()?;
        let result = batch.assign_tag_to_path(relative_path, label);
        let result = result.and_then(|tag| batch.commit().map(|_| tag));
        record_tag_db_event(
            "source_db.assign_tag_to_path",
            self.root(),
            started_at,
            result.as_ref().map(|_| ()),
        );
        result
    }

    /// Remove one normal tag assignment from one wav path in a single transaction.
    pub fn remove_tag_from_path(
        &self,
        relative_path: &Path,
        label: &str,
    ) -> Result<bool, SourceDbError> {
        let started_at = Instant::now();
        let mut batch = self.write_batch()?;
        let result = batch.remove_tag_from_path(relative_path, label);
        let result = result.and_then(|removed| batch.commit().map(|_| removed));
        record_tag_db_event(
            "source_db.remove_tag_from_path",
            self.root(),
            started_at,
            result.as_ref().map(|_| ()),
        );
        result
    }
}

impl<'conn> SourceWriteBatch<'conn> {
    /// Create or resolve a normal tag and assign it to one wav path inside the active batch.
    pub fn assign_tag_to_path(
        &mut self,
        relative_path: &Path,
        label: &str,
    ) -> Result<SourceTag, SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        ensure_wav_path_exists(&self.tx, &path)?;
        let tag = resolve_or_create_tag(&self.tx, label)?;
        self.tx
            .prepare_cached(
                "INSERT OR IGNORE INTO wav_file_tags (path, tag_id)
                 VALUES (?1, ?2)",
            )
            .map_err(map_sql_error)?
            .execute(params![path, tag.id])
            .map_err(map_sql_error)?;
        self.touch_last_curated_at(relative_path)?;
        Ok(tag)
    }

    /// Remove a normal tag assignment from one wav path inside the active batch.
    pub fn remove_tag_from_path(
        &mut self,
        relative_path: &Path,
        label: &str,
    ) -> Result<bool, SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        ensure_wav_path_exists(&self.tx, &path)?;
        let identity = normalize_tag_identity(label)?;
        let removed = self
            .tx
            .prepare_cached(
                "DELETE FROM wav_file_tags
                 WHERE path = ?1
                   AND tag_id = (
                       SELECT id FROM source_tags WHERE normalized_text = ?2
                   )",
            )
            .map_err(map_sql_error)?
            .execute(params![path, identity.normalized_text])
            .map_err(map_sql_error)?
            > 0;
        if removed {
            self.touch_last_curated_at(relative_path)?;
        }
        Ok(removed)
    }

    /// List normal tag display labels for one wav path inside the active batch.
    pub fn tag_labels_for_path(
        &mut self,
        relative_path: &Path,
    ) -> Result<Vec<String>, SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        let mut stmt = self
            .tx
            .prepare_cached(
                "SELECT st.display_label
                 FROM source_tags st
                 JOIN wav_file_tags wft ON wft.tag_id = st.id
                 WHERE wft.path = ?1
                 ORDER BY st.display_label COLLATE NOCASE ASC, st.normalized_text ASC",
            )
            .map_err(map_sql_error)?;
        stmt.query_map([path], |row| row.get::<_, String>(0))
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)
    }

    /// Replace all normal tag assignments for one wav path with labels resolved in this source DB.
    pub fn replace_tags_for_path(
        &mut self,
        relative_path: &Path,
        labels: &[String],
    ) -> Result<(), SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        ensure_wav_path_exists(&self.tx, &path)?;
        self.tx
            .prepare_cached("DELETE FROM wav_file_tags WHERE path = ?1")
            .map_err(map_sql_error)?
            .execute([path.as_str()])
            .map_err(map_sql_error)?;
        for label in labels {
            self.assign_tag_to_path(relative_path, label)?;
        }
        self.touch_last_curated_at(relative_path)?;
        Ok(())
    }

    /// Copy normal tag assignments from one wav path to another inside the same source DB.
    pub fn copy_tags_between_paths(
        &mut self,
        source_relative_path: &Path,
        target_relative_path: &Path,
    ) -> Result<(), SourceDbError> {
        let labels = self.tag_labels_for_path(source_relative_path)?;
        self.replace_tags_for_path(target_relative_path, &labels)
    }
}

fn resolve_or_create_tag(tx: &Transaction<'_>, label: &str) -> Result<SourceTag, SourceDbError> {
    let identity = normalize_tag_identity(label)?;
    tx.prepare_cached(
        "INSERT INTO source_tags (normalized_text, display_label)
         VALUES (?1, ?2)
         ON CONFLICT(normalized_text) DO NOTHING",
    )
    .map_err(map_sql_error)?
    .execute(params![identity.normalized_text, identity.display_label])
    .map_err(map_sql_error)?;
    tag_by_normalized_text(tx, &identity.normalized_text)
}

fn tag_by_normalized_text(
    tx: &Transaction<'_>,
    normalized_text: &str,
) -> Result<SourceTag, SourceDbError> {
    tx.query_row(
        "SELECT id, display_label, normalized_text
         FROM source_tags
         WHERE normalized_text = ?1",
        [normalized_text],
        |row| {
            Ok(SourceTag {
                id: row.get(0)?,
                display_label: row.get(1)?,
                normalized_text: row.get(2)?,
            })
        },
    )
    .map_err(map_sql_error)
}

fn ensure_wav_path_exists(tx: &Transaction<'_>, path: &str) -> Result<(), SourceDbError> {
    let exists = tx
        .query_row(
            "SELECT 1 FROM wav_files WHERE path = ?1",
            [path],
            |_| Ok(()),
        )
        .optional()
        .map_err(map_sql_error)?
        .is_some();
    if exists {
        return Ok(());
    }
    Err(SourceDbError::Unexpected)
}

fn record_tag_db_event(
    operation: &'static str,
    source_root: &Path,
    started_at: Instant,
    result: Result<(), &SourceDbError>,
) {
    let elapsed = started_at.elapsed();
    let source = source_root.display().to_string();
    let error = result.as_ref().err().map(ToString::to_string);
    emit_db_debug_event(DbDebugEvent {
        operation,
        source: Some(&source),
        outcome: if result.is_ok() { "success" } else { "error" },
        elapsed,
        error: error.as_deref(),
    });
}
