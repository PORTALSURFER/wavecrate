use std::collections::{BTreeMap, HashMap};
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
    pub(crate) last_committed_revision: Option<u64>,
}

impl ScanContext {
    pub(super) fn new(
        db: &SourceDatabase,
        mode: ScanMode,
        manifest: Vec<SourceManifestEntry>,
    ) -> Result<Self, ScanError> {
        let existing = index_existing(db)?;
        Ok(Self::from_existing(existing, mode, manifest))
    }

    pub(in crate::sample_sources::scanner) fn from_existing(
        existing: HashMap<PathBuf, WavEntry>,
        mode: ScanMode,
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
            last_committed_revision: None,
        }
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

    pub(in crate::sample_sources::scanner) fn commit_batch(
        &mut self,
        batch: SourceWriteBatch<'_>,
    ) -> Result<u64, ScanError> {
        let (revision, changes) = batch.commit_with_manifest_changes()?;
        for (path, entry) in changes {
            if let Some(entry) = entry {
                self.committed_manifest.insert(path, entry);
            } else {
                self.committed_manifest.remove(&path);
            }
        }
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
