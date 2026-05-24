use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::error::map_app_dir_error;
use super::{LIBRARY_DB_FILE_NAME, LibraryError};
use crate::app_dirs;

pub(super) struct LibraryDatabase {
    pub(super) connection: Connection,
}

impl LibraryDatabase {
    pub(super) fn open() -> Result<Self, LibraryError> {
        let db_path = database_path()?;
        create_parent_if_needed(&db_path)?;
        let connection = Connection::open(&db_path)?;
        let mut db = Self { connection };
        db.apply_pragmas()?;
        db.apply_schema()?;
        db.migrate_analysis_jobs_content_hash()?;
        db.migrate_samples_analysis_metadata()?;
        db.migrate_features_table()?;
        db.migrate_layout_umap_table()?;
        db.migrate_hdbscan_clusters_table()?;
        db.migrate_embeddings_table()?;
        db.migrate_ann_index_meta_table()?;
        Ok(db)
    }

    pub(super) fn into_connection(self) -> Connection {
        self.connection
    }
}

fn database_path() -> Result<PathBuf, LibraryError> {
    app_dirs::app_root_dir()
        .map_err(map_app_dir_error)
        .map(|dir| dir.join(LIBRARY_DB_FILE_NAME))
}

fn create_parent_if_needed(path: &Path) -> Result<(), LibraryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| LibraryError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}
