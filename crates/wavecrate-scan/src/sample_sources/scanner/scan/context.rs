use std::collections::HashMap;
use std::path::PathBuf;

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::WavEntry;

use super::{ScanError, ScanMode, ScanStats};

pub(crate) struct ScanContext {
    pub(crate) existing: HashMap<PathBuf, WavEntry>,
    pub(crate) stats: ScanStats,
    pub(crate) mode: ScanMode,
}

impl ScanContext {
    pub(super) fn new(db: &SourceDatabase, mode: ScanMode) -> Result<Self, ScanError> {
        let existing = index_existing(db)?;
        Ok(Self::from_existing(existing, mode))
    }

    pub(in crate::sample_sources::scanner) fn from_existing(
        existing: HashMap<PathBuf, WavEntry>,
        mode: ScanMode,
    ) -> Self {
        Self {
            existing,
            stats: ScanStats::default(),
            mode,
        }
    }
}

fn index_existing(db: &SourceDatabase) -> Result<HashMap<PathBuf, WavEntry>, ScanError> {
    let entries = db.list_files()?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect())
}
