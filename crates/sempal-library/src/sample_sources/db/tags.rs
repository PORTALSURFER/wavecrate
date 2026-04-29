//! Durable source tag catalog queries and assignment mutations.

use std::path::Path;
use std::time::Instant;

use rusqlite::{OptionalExtension, Transaction, params};

use super::util::{map_sql_error, normalize_relative_path};
use super::{SourceDatabase, SourceDbError, SourceTag, SourceTagUsage, SourceWriteBatch};
use crate::diagnostics::{DbDebugEvent, emit_db_debug_event};

const DEFAULT_TAG_LIMIT: usize = 32;

pub(super) struct NormalizedTagIdentity {
    pub(super) display_label: String,
    pub(super) normalized_text: String,
}

pub(super) fn normalize_tag_identity(label: &str) -> Result<NormalizedTagIdentity, SourceDbError> {
    let display_label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if display_label.is_empty() {
        return Err(SourceDbError::EmptyTagLabel);
    }
    let normalized_text = display_label.to_ascii_lowercase();
    Ok(NormalizedTagIdentity {
        display_label,
        normalized_text,
    })
}

impl SourceDatabase {
    /// Return the most-used persisted normal tags, ordered by usage then label.
    pub fn most_used_tags(&self, limit: usize) -> Result<Vec<SourceTagUsage>, SourceDbError> {
        let limit = query_limit(limit);
        let mut stmt = self
            .connection
            .prepare(
                "SELECT st.id, st.display_label, st.normalized_text, COUNT(wft.path) AS usage_count
                 FROM source_tags st
                 JOIN wav_file_tags wft ON wft.tag_id = st.id
                 GROUP BY st.id, st.display_label, st.normalized_text
                 ORDER BY usage_count DESC,
                          st.display_label COLLATE NOCASE ASC,
                          st.normalized_text ASC
                 LIMIT ?1",
            )
            .map_err(map_sql_error)?;
        collect_tag_usage(
            stmt.query_map([limit], tag_usage_from_row)
                .map_err(map_sql_error)?,
        )
    }

    /// Search persisted normal tags by normalized text. Empty input falls back to most-used tags.
    pub fn search_tags(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SourceTagUsage>, SourceDbError> {
        let identity = match normalize_tag_identity(query) {
            Ok(identity) => identity,
            Err(SourceDbError::EmptyTagLabel) => return self.most_used_tags(limit),
            Err(err) => return Err(err),
        };
        let like_pattern = format!("%{}%", escape_like(&identity.normalized_text));
        let limit = query_limit(limit);
        let mut stmt = self
            .connection
            .prepare(
                "SELECT st.id, st.display_label, st.normalized_text, COUNT(wft.path) AS usage_count
                 FROM source_tags st
                 LEFT JOIN wav_file_tags wft ON wft.tag_id = st.id
                 WHERE st.normalized_text LIKE ?1 ESCAPE '\\'
                 GROUP BY st.id, st.display_label, st.normalized_text
                 ORDER BY CASE WHEN st.normalized_text = ?2 THEN 0 ELSE 1 END,
                          usage_count DESC,
                          st.display_label COLLATE NOCASE ASC,
                          st.normalized_text ASC
                 LIMIT ?3",
            )
            .map_err(map_sql_error)?;
        collect_tag_usage(
            stmt.query_map(
                params![like_pattern, identity.normalized_text, limit],
                |row| tag_usage_from_row(row),
            )
            .map_err(map_sql_error)?,
        )
    }

    /// List normal tags assigned to one wav path.
    pub fn tags_for_path(&self, relative_path: &Path) -> Result<Vec<SourceTag>, SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        let mut stmt = self
            .connection
            .prepare(
                "SELECT st.id, st.display_label, st.normalized_text
                 FROM source_tags st
                 JOIN wav_file_tags wft ON wft.tag_id = st.id
                 WHERE wft.path = ?1
                 ORDER BY st.display_label COLLATE NOCASE ASC, st.normalized_text ASC",
            )
            .map_err(map_sql_error)?;
        let rows = stmt
            .query_map([path], |row| {
                Ok(SourceTag {
                    id: row.get(0)?,
                    display_label: row.get(1)?,
                    normalized_text: row.get(2)?,
                })
            })
            .map_err(map_sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(map_sql_error)
    }

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
            result.as_ref().map(|_| ()).map_err(|err| err),
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
            result.as_ref().map(|_| ()).map_err(|err| err),
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
        Ok(tag)
    }

    /// Remove a normal tag assignment from one wav path inside the active batch.
    pub fn remove_tag_from_path(
        &mut self,
        relative_path: &Path,
        label: &str,
    ) -> Result<bool, SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
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
        Ok(removed)
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
        Ok(())
    } else {
        Err(SourceDbError::Unexpected)
    }
}

fn query_limit(limit: usize) -> i64 {
    limit.max(1).min(DEFAULT_TAG_LIMIT) as i64
}

fn tag_usage_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SourceTagUsage> {
    let assignment_count: i64 = row.get(3)?;
    Ok(SourceTagUsage {
        tag: SourceTag {
            id: row.get(0)?,
            display_label: row.get(1)?,
            normalized_text: row.get(2)?,
        },
        assignment_count: assignment_count.max(0) as u64,
    })
}

fn collect_tag_usage(
    rows: rusqlite::MappedRows<
        '_,
        impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<SourceTagUsage>,
    >,
) -> Result<Vec<SourceTagUsage>, SourceDbError> {
    rows.collect::<Result<Vec<_>, _>>().map_err(map_sql_error)
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::tempdir;

    use super::super::SourceDatabase;

    #[test]
    fn normalization_prevents_case_and_spacing_duplicates() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
        db.upsert_file(Path::new("two.wav"), 10, 5).unwrap();

        let first = db
            .assign_tag_to_path(Path::new("one.wav"), "  Deep   Kick ")
            .unwrap();
        let second = db
            .assign_tag_to_path(Path::new("two.wav"), "deep kick")
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.display_label, "Deep Kick");
        assert_eq!(first.normalized_text, "deep kick");
        assert_eq!(db.search_tags("DEEP    KICK", 8).unwrap().len(), 1);
    }

    #[test]
    fn most_used_tags_order_by_persisted_usage_then_label() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        for path in ["one.wav", "two.wav", "three.wav", "four.wav"] {
            db.upsert_file(Path::new(path), 10, 5).unwrap();
        }
        db.assign_tag_to_path(Path::new("one.wav"), "zeta").unwrap();
        db.assign_tag_to_path(Path::new("one.wav"), "alpha")
            .unwrap();
        db.assign_tag_to_path(Path::new("two.wav"), "alpha")
            .unwrap();
        db.assign_tag_to_path(Path::new("three.wav"), "beta")
            .unwrap();
        db.assign_tag_to_path(Path::new("four.wav"), "beta")
            .unwrap();

        let labels = db
            .most_used_tags(8)
            .unwrap()
            .into_iter()
            .map(|usage| (usage.tag.display_label, usage.assignment_count))
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                ("alpha".to_string(), 2),
                ("beta".to_string(), 2),
                ("zeta".to_string(), 1),
            ]
        );
    }

    #[test]
    fn assignment_api_creates_then_resolves_existing_tag() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
        db.upsert_file(Path::new("two.wav"), 10, 5).unwrap();

        let created = db
            .assign_tag_to_path(Path::new("one.wav"), "Texture")
            .unwrap();
        let resolved = db
            .assign_tag_to_path(Path::new("two.wav"), " texture ")
            .unwrap();

        assert_eq!(created.id, resolved.id);
        assert_eq!(
            db.tags_for_path(Path::new("two.wav")).unwrap(),
            vec![created]
        );
        let removed = db
            .remove_tag_from_path(Path::new("two.wav"), "TEXTURE")
            .unwrap();
        assert!(removed);
        assert!(db.tags_for_path(Path::new("two.wav")).unwrap().is_empty());
    }

    #[test]
    fn search_tags_orders_exact_match_before_usage_matches() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        for path in ["one.wav", "two.wav", "three.wav"] {
            db.upsert_file(Path::new(path), 10, 5).unwrap();
        }
        db.assign_tag_to_path(Path::new("one.wav"), "deep kick")
            .unwrap();
        db.assign_tag_to_path(Path::new("two.wav"), "kick").unwrap();
        db.assign_tag_to_path(Path::new("three.wav"), "deep kick")
            .unwrap();

        let labels = db
            .search_tags("kick", 8)
            .unwrap()
            .into_iter()
            .map(|usage| usage.tag.display_label)
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["kick".to_string(), "deep kick".to_string()]);
    }
}
