use std::collections::HashMap;
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
    pub(crate) last_committed_snapshot: Option<(u64, Vec<SourceManifestEntry>)>,
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
            rename_candidate_generation: None,
            last_committed_snapshot: None,
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
    ) -> Result<(u64, Vec<SourceManifestEntry>), ScanError> {
        let snapshot = batch.commit_with_manifest_snapshot()?;
        self.last_committed_snapshot = Some(snapshot.clone());
        Ok(snapshot)
    }
}

fn index_existing(db: &SourceDatabase) -> Result<HashMap<PathBuf, WavEntry>, ScanError> {
    let entries = db.list_files()?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.relative_path.clone(), entry))
        .collect())
}
