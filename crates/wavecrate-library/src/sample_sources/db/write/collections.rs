use std::path::Path;

use rusqlite::{OptionalExtension, params};

use super::super::util::map_sql_error;
use super::super::{SampleCollection, SourceDbError, SourceWriteBatch};
use super::mutation::{update_path_i64_statement, update_path_null_statement};

const UPDATE_COLLECTION_SQL: &str = "UPDATE wav_files SET collection = ?1 WHERE path = ?2";
const CLEAR_COLLECTION_SQL: &str = "UPDATE wav_files SET collection = NULL WHERE path = ?1";

impl SourceWriteBatch<'_> {
    /// Replace every fixed collection slot for a wav row.
    pub fn set_collection(
        &mut self,
        relative_path: &Path,
        collection: Option<SampleCollection>,
    ) -> Result<(), SourceDbError> {
        let path_str = super::super::normalize_relative_path(relative_path)?;
        self.tx
            .execute(
                "DELETE FROM wav_file_collections WHERE path = ?1",
                params![path_str.as_str()],
            )
            .map_err(map_sql_error)?;
        if let Some(collection) = collection {
            self.insert_collection_membership(path_str.as_str(), collection)?;
        }
        self.persist_legacy_collection(relative_path, collection)
    }

    /// Add one fixed collection slot for a wav row without clearing other slots.
    pub fn add_collection(
        &mut self,
        relative_path: &Path,
        collection: SampleCollection,
    ) -> Result<(), SourceDbError> {
        let path_str = super::super::normalize_relative_path(relative_path)?;
        self.insert_collection_membership(path_str.as_str(), collection)?;
        self.tx
            .execute(
                "UPDATE wav_files
                 SET collection = COALESCE(collection, ?1)
                 WHERE path = ?2",
                params![collection.as_i64(), path_str.as_str()],
            )
            .map_err(map_sql_error)?;
        self.touch_last_curated_at(relative_path)
    }

    /// Remove one fixed collection slot for a wav row without clearing other slots.
    pub fn remove_collection(
        &mut self,
        relative_path: &Path,
        collection: SampleCollection,
    ) -> Result<(), SourceDbError> {
        let path_str = super::super::normalize_relative_path(relative_path)?;
        self.tx
            .execute(
                "DELETE FROM wav_file_collections
                 WHERE path = ?1 AND collection = ?2",
                params![path_str.as_str(), collection.as_i64()],
            )
            .map_err(map_sql_error)?;
        let first_remaining = self
            .tx
            .query_row(
                "SELECT collection
                 FROM wav_file_collections
                 WHERE path = ?1
                 ORDER BY collection ASC
                 LIMIT 1",
                params![path_str.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .and_then(SampleCollection::from_i64);
        self.persist_legacy_collection(relative_path, first_remaining)
    }

    fn persist_legacy_collection(
        &mut self,
        relative_path: &Path,
        collection: Option<SampleCollection>,
    ) -> Result<(), SourceDbError> {
        match collection {
            Some(collection) => update_path_i64_statement(
                &self.tx,
                UPDATE_COLLECTION_SQL,
                relative_path,
                collection.as_i64(),
            )?,
            None => update_path_null_statement(&self.tx, CLEAR_COLLECTION_SQL, relative_path)?,
        }
        self.touch_last_curated_at(relative_path)
    }

    fn insert_collection_membership(
        &self,
        path: &str,
        collection: SampleCollection,
    ) -> Result<(), SourceDbError> {
        self.tx
            .execute(
                "INSERT OR IGNORE INTO wav_file_collections (path, collection)
                 VALUES (?1, ?2)",
                params![path, collection.as_i64()],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }
}
