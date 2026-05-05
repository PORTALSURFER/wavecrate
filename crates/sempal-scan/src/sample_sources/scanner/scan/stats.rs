use std::path::PathBuf;

/// Summary of a scan run.
#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    /// Number of newly discovered files.
    pub added: usize,
    /// Number of files updated in-place.
    pub updated: usize,
    /// Number of files now missing from disk.
    pub missing: usize,
    /// Total number of files scanned.
    pub total_files: usize,
    /// Number of files with changed content hashes.
    pub content_changed: usize,
    /// Number of files whose content hashes were computed during the scan.
    pub hashes_computed: usize,
    /// Number of files whose content hashes were deferred during the scan.
    pub hashes_pending: usize,
    /// Number of missing rows reconciled to renamed files.
    pub renames_reconciled: usize,
    /// Detailed list of files whose source-visible metadata was updated in place.
    pub updated_samples: Vec<UpdatedSample>,
    /// Detailed list of source-visible rename reconciliations.
    pub renamed_samples: Vec<RenamedSample>,
    /// Detailed list of changed samples.
    pub changed_samples: Vec<ChangedSample>,
}

/// Metadata describing a sample whose tracked file facts changed without moving.
#[derive(Debug, Clone)]
pub struct UpdatedSample {
    /// Path relative to the source root.
    pub relative_path: PathBuf,
    /// File size in bytes.
    pub file_size: u64,
    /// Last modified timestamp in epoch nanoseconds.
    pub modified_ns: i64,
    /// Updated content hash when the scan computed one.
    pub content_hash: Option<String>,
}

/// Metadata describing a sample whose path was reconciled as a rename.
#[derive(Debug, Clone)]
pub struct RenamedSample {
    /// Previous path relative to the source root.
    pub old_relative_path: PathBuf,
    /// Current path relative to the source root.
    pub new_relative_path: PathBuf,
    /// File size in bytes at the current path.
    pub file_size: u64,
    /// Last modified timestamp in epoch nanoseconds at the current path.
    pub modified_ns: i64,
    /// Updated content hash when the scan computed or reused one.
    pub content_hash: Option<String>,
}

/// Metadata describing a sample whose content changed.
#[derive(Debug, Clone)]
pub struct ChangedSample {
    /// Path relative to the source root.
    pub relative_path: PathBuf,
    /// File size in bytes.
    pub file_size: u64,
    /// Last modified timestamp in epoch nanoseconds.
    pub modified_ns: i64,
    /// Updated content hash.
    pub content_hash: String,
}
