use std::collections::HashMap;
use std::path::PathBuf;

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::WavEntry;

use super::{ScanError, ScanMode, ScanStats};

pub(crate) struct ScanContext {
    pub(crate) existing: HashMap<PathBuf, WavEntry>,
    /// On-demand rename candidate paths keyed by content hash.
    ///
    /// Unlike the previous full in-memory hash index, this cache only stores
    /// keys encountered during the current walk.
    pub(crate) rename_candidates_by_hash: HashMap<String, Vec<PathBuf>>,
    /// On-demand rename candidate paths keyed by `(file_size, modified_ns)`.
    ///
    /// This keeps quick-scan reconciliation incremental and avoids triplicating
    /// all row mappings in memory.
    pub(crate) rename_candidates_by_facts: HashMap<(u64, i64), Vec<PathBuf>>,
    pub(crate) stats: ScanStats,
    pub(crate) mode: ScanMode,
}

impl ScanContext {
    pub(super) fn new(db: &SourceDatabase, mode: ScanMode) -> Result<Self, ScanError> {
        let existing = index_existing(db)?;
        Ok(Self {
            existing,
            rename_candidates_by_hash: HashMap::new(),
            rename_candidates_by_facts: HashMap::new(),
            stats: ScanStats::default(),
            mode,
        })
    }
}

fn index_existing(db: &SourceDatabase) -> Result<HashMap<PathBuf, WavEntry>, ScanError> {
    let entries = db.list_files()?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect())
}
