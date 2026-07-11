use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::util::{map_sql_error, normalize_relative_path, parse_relative_path_from_db};
use super::{SourceDatabase, SourceDbError, SourceWriteBatch};

const TARGETED_SCAN_GENERATION_KEY: &str = "targeted_scan_generation_v1";

impl SourceDatabase {
    /// List destination paths discovered in the current or immediately previous targeted scan.
    pub fn list_pending_rename_destinations(&self) -> Result<Vec<PathBuf>, SourceDbError> {
        let generation = self
            .get_metadata(TARGETED_SCAN_GENERATION_KEY)?
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let oldest = generation.saturating_sub(1);
        let mut statement = self
            .connection
            .prepare(
                "SELECT path
                 FROM pending_wav_rename_destinations
                 WHERE scan_generation >= ?1
                 ORDER BY path ASC",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![oldest as i64], |row| row.get::<_, String>(0))
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        Ok(rows
            .into_iter()
            .filter_map(|path| match parse_relative_path_from_db(&path) {
                Ok(path) => Some(path),
                Err(error) => {
                    tracing::warn!(%error, "Skipping invalid pending rename destination path");
                    None
                }
            })
            .collect())
    }
}

impl SourceWriteBatch<'_> {
    /// Begin a targeted watcher scan and expire candidates older than one prior batch.
    pub fn begin_targeted_scan_generation(&mut self) -> Result<u64, SourceDbError> {
        let current = self
            .tx
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![TARGETED_SCAN_GENERATION_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        let generation = current.saturating_add(1);
        self.set_metadata(TARGETED_SCAN_GENERATION_KEY, &generation.to_string())?;
        self.tx
            .execute(
                "DELETE FROM pending_wav_rename_destinations WHERE scan_generation < ?1",
                params![generation.saturating_sub(1) as i64],
            )
            .map_err(map_sql_error)?;
        Ok(generation)
    }

    /// Start a full quick-scan candidate snapshot at the current watcher generation.
    pub fn begin_quick_scan_rename_candidates(&mut self) -> Result<u64, SourceDbError> {
        let generation = self
            .tx
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![TARGETED_SCAN_GENERATION_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        self.clear_all_pending_rename_destinations()?;
        Ok(generation)
    }

    /// Retain a newly discovered targeted path across the next watcher batch.
    pub fn stage_pending_rename_destination(
        &mut self,
        relative_path: &Path,
        generation: u64,
    ) -> Result<(), SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        self.tx
            .execute(
                "INSERT INTO pending_wav_rename_destinations (path, scan_generation)
                 VALUES (?1, ?2)
                 ON CONFLICT(path) DO UPDATE SET scan_generation = excluded.scan_generation",
                params![path, generation as i64],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Forget a destination after it has been consumed by proven rename reconciliation.
    pub fn clear_pending_rename_destination(
        &mut self,
        relative_path: &Path,
    ) -> Result<(), SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        self.tx
            .execute(
                "DELETE FROM pending_wav_rename_destinations WHERE path = ?1",
                params![path],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Drop all retained destination candidates during authoritative rebuilds.
    pub fn clear_all_pending_rename_destinations(&mut self) -> Result<(), SourceDbError> {
        self.tx
            .execute("DELETE FROM pending_wav_rename_destinations", [])
            .map_err(map_sql_error)?;
        Ok(())
    }
}
