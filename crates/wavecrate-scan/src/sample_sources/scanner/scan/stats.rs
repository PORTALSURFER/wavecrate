use std::path::PathBuf;

use wavecrate_library::sample_sources::SourceManifestEntry;
use wavecrate_library::sample_sources::db::ContentAuditReport;

/// One non-audio regular file observed during the authoritative source traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTreeFile {
    /// File path relative to the source root.
    pub relative_path: PathBuf,
    /// File size observed without following symbolic links.
    pub file_size: u64,
}

/// Browser layout facts captured by the same traversal that reconciles the source manifest.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourceTreeSnapshot {
    /// All visible directories, relative to the source root, including the empty root path.
    pub directories: Vec<PathBuf>,
    /// Visible regular files that are not authoritative supported-audio manifest rows.
    pub other_files: Vec<SourceTreeFile>,
    /// Bounded diagnostics for entries that could not be classified or enumerated.
    pub diagnostics: Vec<String>,
    /// Relative directory or entry prefixes whose descendants were not
    /// authoritatively observed during this traversal.
    ///
    /// This is internal scan state carried to missing-row reconciliation. It
    /// is deliberately unbounded: dropping a prefix here could turn an I/O
    /// failure into a false deletion.
    #[doc(hidden)]
    pub uncertain_prefixes: Vec<PathBuf>,
}

impl SourceTreeSnapshot {
    /// A projection is safe to publish only when every encountered entry was classified.
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Summary of a scan run.
#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    /// Authoritative identity delta observed at the final committed source revision.
    pub committed_delta: CommittedSourceDelta,
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
    /// Durable content-verification coverage after this scan.
    pub content_audit: Option<ContentAuditReport>,
    /// Number of missing rows reconciled to renamed files.
    pub renames_reconciled: usize,
    /// Detailed list of files whose source-visible metadata was updated in place.
    pub updated_samples: Vec<UpdatedSample>,
    /// Detailed list of source-visible rename reconciliations.
    pub renamed_samples: Vec<RenamedSample>,
    /// Detailed list of changed samples.
    pub changed_samples: Vec<ChangedSample>,
    /// Newly inserted paths from this scan that are eligible as rename destinations.
    #[doc(hidden)]
    pub rename_candidate_paths: Vec<PathBuf>,
    #[doc(hidden)]
    pub manifest_before: Vec<SourceManifestEntry>,
    #[doc(hidden)]
    pub manifest_after: Vec<SourceManifestEntry>,
    /// Filesystem layout captured by the authoritative full traversal.
    #[doc(hidden)]
    pub source_tree_snapshot: Option<SourceTreeSnapshot>,
}

impl ScanStats {
    pub(super) fn merge_deferred_hashes(&mut self, mut deferred: Self) {
        self.hashes_computed += deferred.hashes_computed;
        self.content_audit = deferred.content_audit.take().or(self.content_audit.take());
        self.hashes_pending = self.hashes_pending.saturating_sub(deferred.hashes_computed);
        self.renames_reconciled += deferred.renames_reconciled;
        self.updated_samples.append(&mut deferred.updated_samples);
        self.renamed_samples.append(&mut deferred.renamed_samples);
        self.changed_samples.append(&mut deferred.changed_samples);
        if !deferred.manifest_after.is_empty() || deferred.committed_delta.revision > 0 {
            self.manifest_after = deferred.manifest_after;
            self.committed_delta = super::super::manifest::build_committed_delta(
                &self.manifest_before,
                &self.manifest_after,
                deferred.committed_delta.revision,
            );
        }
        if deferred.source_tree_snapshot.is_some() {
            self.source_tree_snapshot = deferred.source_tree_snapshot;
        }
    }

    pub(crate) fn record_rename_candidate(&mut self, path: PathBuf) {
        self.rename_candidate_paths.push(path);
    }
}

/// One current or retired identity in a committed source-manifest delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestIdentityDelta {
    /// Stable identity used to fence downstream work.
    pub identity: String,
    /// Source-relative path at this revision.
    pub relative_path: PathBuf,
    /// Full hash or explicit pending generation for this identity.
    pub content_generation: String,
    /// Whether source-visible size or modification metadata changed.
    pub source_metadata_changed: bool,
}

/// One identity whose committed source-relative path changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedManifestIdentity {
    /// Stable identity used to fence downstream work.
    pub identity: String,
    /// Previous source-relative path.
    pub old_relative_path: PathBuf,
    /// Current source-relative path.
    pub new_relative_path: PathBuf,
    /// Current full hash or explicit pending generation.
    pub content_generation: String,
}

/// Structured source-manifest delta published only after the authoritative commit.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommittedSourceDelta {
    /// Monotonic committed source-path revision.
    pub revision: u64,
    /// Identities newly present at this revision.
    pub created: Vec<ManifestIdentityDelta>,
    /// Identities whose content generation changed at this revision.
    pub changed: Vec<ManifestIdentityDelta>,
    /// Identities whose path changed without losing stable ownership.
    pub moved: Vec<MovedManifestIdentity>,
    /// Identities no longer present at this revision.
    pub deleted: Vec<ManifestIdentityDelta>,
}

impl CommittedSourceDelta {
    /// Return true when the committed manifest did not change.
    pub fn is_empty(&self) -> bool {
        self.created.is_empty()
            && self.changed.is_empty()
            && self.moved.is_empty()
            && self.deleted.is_empty()
    }
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
