use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::{SourceWriteBatch, WavEntry};
use wavecrate_library::sample_sources::SourceManifestEntry;

use super::{ScanError, ScanMode, ScanStats};

pub(crate) struct ScanContext {
    pub(crate) existing: HashMap<PathBuf, WavEntry>,
    pub(crate) stats: ScanStats,
    pub(crate) mode: ScanMode,
    pub(crate) rename_candidate_generation: Option<u64>,
    committed_manifest: BTreeMap<PathBuf, SourceManifestEntry>,
    committed_manifest_revision: u64,
    pub(crate) last_committed_revision: Option<u64>,
    manifest_audit: Option<ManifestAuditCheckpoint>,
    source_tree_incomplete: bool,
}

struct ManifestAuditCheckpoint {
    previously_checked: HashSet<PathBuf>,
    pending: Vec<PathBuf>,
    expected_total: usize,
}

impl ScanContext {
    pub(super) fn new(
        db: &SourceDatabase,
        mode: ScanMode,
        manifest_revision: u64,
        manifest: Vec<SourceManifestEntry>,
    ) -> Result<Self, ScanError> {
        let existing = index_existing(db)?;
        Ok(Self::from_existing(
            existing,
            mode,
            manifest_revision,
            manifest,
        ))
    }

    pub(in crate::sample_sources::scanner) fn from_existing(
        existing: HashMap<PathBuf, WavEntry>,
        mode: ScanMode,
        manifest_revision: u64,
        manifest: Vec<SourceManifestEntry>,
    ) -> Self {
        Self {
            existing,
            stats: ScanStats::default(),
            mode,
            rename_candidate_generation: None,
            committed_manifest: manifest
                .into_iter()
                .map(|entry| (entry.relative_path.clone(), entry))
                .collect(),
            committed_manifest_revision: manifest_revision,
            last_committed_revision: None,
            manifest_audit: None,
            source_tree_incomplete: false,
        }
    }

    pub(in crate::sample_sources::scanner) fn mark_source_tree_incomplete(&mut self) {
        self.source_tree_incomplete = true;
    }

    pub(in crate::sample_sources::scanner) fn source_tree_incomplete(&self) -> bool {
        self.source_tree_incomplete
    }

    pub(in crate::sample_sources::scanner) fn resume_manifest_audit(
        &mut self,
        db: &SourceDatabase,
        started_at: i64,
    ) -> Result<(), ScanError> {
        let previously_checked = db
            .begin_or_resume_manifest_audit(started_at)?
            .into_iter()
            .collect::<HashSet<_>>();
        self.stats.total_files = previously_checked.len();
        self.manifest_audit = Some(ManifestAuditCheckpoint {
            expected_total: self.committed_manifest.len().max(previously_checked.len()),
            previously_checked,
            pending: Vec::new(),
        });
        Ok(())
    }

    pub(in crate::sample_sources::scanner) fn manifest_audit_progress(
        &self,
    ) -> Option<(usize, usize)> {
        self.manifest_audit.as_ref().map(|audit| {
            (
                self.stats.total_files,
                audit.expected_total.max(self.stats.total_files),
            )
        })
    }

    pub(in crate::sample_sources::scanner) fn skip_previously_audited_path(
        &mut self,
        relative_path: &std::path::Path,
    ) -> bool {
        let already_checked = self
            .manifest_audit
            .as_ref()
            .is_some_and(|audit| audit.previously_checked.contains(relative_path));
        if already_checked {
            self.existing.remove(relative_path);
        }
        already_checked
    }

    pub(in crate::sample_sources::scanner) fn record_manifest_audit_paths(
        &mut self,
        db: &SourceDatabase,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Result<(), ScanError> {
        let Some(audit) = self.manifest_audit.as_mut() else {
            return Ok(());
        };
        for path in paths {
            if audit.previously_checked.insert(path.clone()) {
                audit.pending.push(path);
                self.stats.total_files = self.stats.total_files.saturating_add(1);
            }
        }
        if audit.pending.len() >= 64 {
            db.checkpoint_manifest_audit_paths(&audit.pending)?;
            audit.pending.clear();
        }
        Ok(())
    }

    pub(in crate::sample_sources::scanner) fn flush_manifest_audit_checkpoint(
        &mut self,
        db: &SourceDatabase,
    ) -> Result<(), ScanError> {
        let Some(audit) = self.manifest_audit.as_mut() else {
            return Ok(());
        };
        db.checkpoint_manifest_audit_paths(&audit.pending)?;
        audit.pending.clear();
        Ok(())
    }

    pub(in crate::sample_sources::scanner) fn resumable_manifest_audit_active(&self) -> bool {
        self.manifest_audit.is_some()
    }

    pub(in crate::sample_sources::scanner) fn ensure_rename_candidate_generation(
        &mut self,
        batch: &mut SourceWriteBatch<'_>,
    ) -> Result<(), ScanError> {
        if self.rename_candidate_generation.is_some() || self.mode == ScanMode::Hard {
            return Ok(());
        }
        let generation = match self.mode {
            ScanMode::Targeted => batch.begin_targeted_scan_generation()?,
            ScanMode::Quick => batch.begin_quick_scan_rename_candidates()?,
            ScanMode::Hard => unreachable!("hard scans do not track rename destinations"),
        };
        self.rename_candidate_generation = Some(generation);
        Ok(())
    }

    pub(in crate::sample_sources::scanner) fn committed_file_identity(
        &self,
        relative_path: &std::path::Path,
    ) -> Option<&str> {
        self.committed_manifest
            .get(relative_path)
            .and_then(|entry| entry.file_identity.as_deref())
    }

    pub(in crate::sample_sources::scanner) fn commit_batch(
        &mut self,
        batch: SourceWriteBatch<'_>,
    ) -> Result<u64, ScanError> {
        self.commit_batch_with_post_commit_hook(batch, || {})
    }

    fn commit_batch_with_post_commit_hook(
        &mut self,
        batch: SourceWriteBatch<'_>,
        post_commit_hook: impl FnOnce(),
    ) -> Result<u64, ScanError> {
        let (revision, changes, snapshot) =
            batch.commit_with_manifest_changes(self.committed_manifest_revision)?;
        post_commit_hook();
        if let Some(snapshot) = snapshot {
            self.committed_manifest = snapshot
                .into_iter()
                .map(|entry| (entry.relative_path.clone(), entry))
                .collect();
        } else {
            for (path, entry) in changes {
                if let Some(entry) = entry {
                    self.committed_manifest.insert(path, entry);
                } else {
                    self.committed_manifest.remove(&path);
                }
            }
        }
        self.committed_manifest_revision = revision;
        self.last_committed_revision = Some(revision);
        Ok(revision)
    }

    pub(in crate::sample_sources::scanner) fn committed_snapshot(
        &self,
        revision: u64,
    ) -> (u64, Vec<SourceManifestEntry>) {
        (
            revision,
            self.committed_manifest.values().cloned().collect(),
        )
    }
}

fn index_existing(db: &SourceDatabase) -> Result<HashMap<PathBuf, WavEntry>, ScanError> {
    let entries = db.list_files()?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn interleaved_manifest_writer_forces_exact_committed_resnapshot() {
        let directory = tempfile::tempdir().expect("source root");
        let database = SourceDatabase::open_for_scan(directory.path()).expect("source database");
        let mut initial_batch = database.write_batch().expect("initial batch");
        initial_batch
            .upsert_file_with_hash(Path::new("initial.wav"), 1, 1, "initial")
            .expect("initial row");
        initial_batch.commit().expect("commit initial row");
        let (manifest_revision, manifest) = database
            .manifest_snapshot_with_revision()
            .expect("initial manifest");
        let existing = index_existing(&database).expect("existing rows");
        let mut context =
            ScanContext::from_existing(existing, ScanMode::Quick, manifest_revision, manifest);

        let mut external_batch = database.write_batch().expect("external batch");
        external_batch
            .upsert_file_with_hash(Path::new("external.wav"), 2, 2, "external")
            .expect("interleaved writer");
        external_batch.commit().expect("commit interleaved writer");
        let mut scan_batch = database.write_batch().expect("scan batch");
        scan_batch
            .upsert_file_with_hash(Path::new("scan.wav"), 3, 3, "scan")
            .expect("scan row");
        let revision = context
            .commit_batch_with_post_commit_hook(scan_batch, || {
                let mut later_batch = database.write_batch().expect("later batch");
                later_batch
                    .upsert_file_with_hash(Path::new("later.wav"), 4, 4, "later")
                    .expect("later row");
                later_batch.commit().expect("commit later writer");
            })
            .expect("commit scan batch");
        let (_revision, snapshot) = context.committed_snapshot(revision);
        let paths = snapshot
            .into_iter()
            .map(|entry| entry.relative_path)
            .collect::<Vec<_>>();

        assert!(revision < database.get_revision().expect("current revision"));
        assert_eq!(
            paths,
            vec![
                PathBuf::from("external.wav"),
                PathBuf::from("initial.wav"),
                PathBuf::from("scan.wav"),
            ]
        );
    }
}
