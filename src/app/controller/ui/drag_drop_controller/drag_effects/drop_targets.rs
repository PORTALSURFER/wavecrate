use super::super::DragDropController;
use super::move_transaction::{move_sample_file, prepare_staged_copy, prepare_staged_move};
use crate::app::controller::StatusTone;
use crate::app::state::DragSample;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{Rating, SourceId, WavEntry};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

mod transactions;

use transactions::{
    clear_file_op_journal_entry, register_drop_target_target_entry,
    rollback_staged_copy, rollback_staged_move, rollback_staged_move_after_target_db_stage,
    sample_move_metadata, warn_on_journal_stage_update,
};

/// Metadata copied from the dragged sample onto the copied/moved target entry.
#[derive(Clone, Copy)]
struct DroppedSampleMetadata {
    tag: Rating,
    looped: bool,
    locked: bool,
    last_played_at: Option<i64>,
}

/// Inputs for copying one dragged sample into another source/folder target.
struct CopySampleTargetRequest<'a> {
    absolute: &'a Path,
    target: &'a crate::sample_sources::SampleSource,
    target_folder: &'a Path,
    file_name: &'a std::ffi::OsStr,
    metadata: DroppedSampleMetadata,
}

impl DragDropController<'_> {
    pub(crate) fn handle_sample_drop_to_drop_target(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        target_path: PathBuf,
        copy_requested: bool,
    ) {
        info!(
            "Drop target requested: source_id={:?} path={} target={}",
            source_id,
            relative_path.display(),
            target_path.display()
        );
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .cloned()
        else {
            warn!("Drop target: missing source {:?}", source_id);
            self.set_status("Source not available for drop", StatusTone::Error);
            return;
        };
        let Some(target) = self.resolve_drop_target_location(&target_path) else {
            self.set_status(
                "Drop target is not inside a configured source",
                StatusTone::Warning,
            );
            return;
        };
        let target_dir = target.source.root.join(&target.relative_folder);
        if !target_dir.is_dir() {
            self.set_status(
                format!("Drop target missing: {}", target_dir.display()),
                StatusTone::Error,
            );
            return;
        }
        if source.id == target.source.id && !copy_requested {
            self.handle_sample_drop_to_folder(source_id, relative_path, &target.relative_folder);
            return;
        }

        let file_name = match relative_path.file_name() {
            Some(name) => name.to_owned(),
            None => {
                self.set_status("Sample name unavailable for drop", StatusTone::Error);
                return;
            }
        };
        let absolute = source.root.join(&relative_path);
        if !absolute.exists() {
            self.set_status(
                format!("File missing: {}", relative_path.display()),
                StatusTone::Error,
            );
            return;
        }
        let tag = match self.sample_tag_for(&source, &relative_path) {
            Ok(tag) => tag,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return;
            }
        };
        let looped = match self.sample_looped_for(&source, &relative_path) {
            Ok(looped) => looped,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return;
            }
        };
        let locked = self
            .wav_index_for_path(&relative_path)
            .and_then(|idx| self.wav_entries.entry(idx))
            .map(|entry| entry.locked)
            .unwrap_or(false);
        let last_played_at = self
            .sample_last_played_for(&source, &relative_path)
            .unwrap_or(None);
        let metadata = DroppedSampleMetadata {
            tag,
            looped,
            locked,
            last_played_at,
        };
        if copy_requested {
            match self.copy_sample_to_target(CopySampleTargetRequest {
                absolute: &absolute,
                target: &target.source,
                target_folder: &target.relative_folder,
                file_name: &file_name,
                metadata,
            }) {
                Ok(path) => {
                    self.set_status(format!("Copied to {}", path.display()), StatusTone::Info);
                }
                Err(err) => self.set_status(err, StatusTone::Error),
            }
            return;
        }

        let destination_relative =
            match move_destination_relative(&target.source, &target.relative_folder, &file_name) {
                Ok(path) => path,
                Err(err) => {
                    self.set_status(err, StatusTone::Error);
                    return;
                }
            };
        let target_db = match self.database_for(&target.source) {
            Ok(db) => db,
            Err(err) => {
                self.set_status(format!("Database unavailable: {err}"), StatusTone::Error);
                return;
            }
        };
        let prepared = match prepare_staged_move(
            target_db.as_ref(),
            &source.root,
            &relative_path,
            &target.source.root,
            &destination_relative,
            sample_move_metadata(metadata),
        ) {
            Ok(prepared) => prepared,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return;
            }
        };
        if let Err(err) = register_drop_target_target_entry(
            target_db.as_ref(),
            &destination_relative,
            prepared.file_size,
            prepared.modified_ns,
            metadata,
        ) {
            rollback_staged_move(target_db.as_ref(), &prepared);
            self.set_status(err, StatusTone::Error);
            return;
        }
        warn_on_journal_stage_update(
            target_db.as_ref(),
            &prepared.op_id,
            file_ops_journal::FileOpStage::TargetDb,
        );
        let source_db = match self.database_for(&source) {
            Ok(db) => db,
            Err(err) => {
                rollback_staged_move_after_target_db_stage(
                    target_db.as_ref(),
                    &prepared,
                    &destination_relative,
                );
                self.set_status(format!("Database unavailable: {err}"), StatusTone::Error);
                return;
            }
        };
        if let Err(err) = source_db.remove_file(&relative_path) {
            rollback_staged_move_after_target_db_stage(
                target_db.as_ref(),
                &prepared,
                &destination_relative,
            );
            self.set_status(format!("Failed to drop database row: {err}"), StatusTone::Error);
            return;
        }
        warn_on_journal_stage_update(
            target_db.as_ref(),
            &prepared.op_id,
            file_ops_journal::FileOpStage::SourceDb,
        );
        if let Err(err) = move_sample_file(&prepared.staged_absolute, &prepared.target_absolute) {
            self.set_status(format!("Failed to finalize move: {err}"), StatusTone::Error);
            return;
        }
        clear_file_op_journal_entry(target_db.as_ref(), &prepared.op_id);
        self.prune_cached_sample(&source, &relative_path);
        self.insert_cached_entry(
            &target.source,
            WavEntry {
                relative_path: destination_relative.clone(),
                file_size: prepared.file_size,
                modified_ns: prepared.modified_ns,
                content_hash: None,
                tag: metadata.tag,
                looped: metadata.looped,
                locked: metadata.locked,
                missing: false,
                last_played_at: metadata.last_played_at,
            },
        );
        self.set_status(
            format!("Moved to {}", target_dir.display()),
            StatusTone::Info,
        );
    }

    pub(crate) fn handle_samples_drop_to_drop_target(
        &mut self,
        samples: &[DragSample],
        target_path: PathBuf,
        copy_requested: bool,
    ) {
        for sample in samples {
            self.handle_sample_drop_to_drop_target(
                sample.source_id.clone(),
                sample.relative_path.clone(),
                target_path.clone(),
                copy_requested,
            );
        }
    }

    /// Copy one dragged sample into the resolved target folder and register it in caches/DB.
    fn copy_sample_to_target(
        &mut self,
        request: CopySampleTargetRequest<'_>,
    ) -> Result<PathBuf, String> {
        let CopySampleTargetRequest {
            absolute,
            target,
            target_folder,
            file_name,
            metadata,
        } = request;
        let destination_relative = copy_destination_relative(target, target_folder, file_name)?;
        let target_db = self
            .database_for(target)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let prepared = prepare_staged_copy(
            target_db.as_ref(),
            absolute,
            &target.root,
            &destination_relative,
            sample_move_metadata(metadata),
        )?;
        if let Err(err) = register_drop_target_target_entry(
            target_db.as_ref(),
            &destination_relative,
            prepared.file_size,
            prepared.modified_ns,
            metadata,
        ) {
            rollback_staged_copy(target_db.as_ref(), &prepared);
            return Err(err);
        }
        warn_on_journal_stage_update(
            target_db.as_ref(),
            &prepared.op_id,
            file_ops_journal::FileOpStage::TargetDb,
        );
        if let Err(err) = move_sample_file(&prepared.staged_absolute, &prepared.target_absolute) {
            return Err(format!("Failed to finalize copy: {err}"));
        }
        clear_file_op_journal_entry(target_db.as_ref(), &prepared.op_id);
        self.insert_cached_entry(
            target,
            WavEntry {
                relative_path: destination_relative.clone(),
                file_size: prepared.file_size,
                modified_ns: prepared.modified_ns,
                content_hash: None,
                tag: metadata.tag,
                looped: metadata.looped,
                locked: metadata.locked,
                missing: false,
                last_played_at: metadata.last_played_at,
            },
        );
        Ok(destination_relative)
    }
}

fn move_destination_relative(
    target: &crate::sample_sources::SampleSource,
    target_folder: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<PathBuf, String> {
    let relative = if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    };
    let destination = target.root.join(&relative);
    if destination.exists() {
        return Err(format!(
            "A file already exists at {}",
            destination.display()
        ));
    }
    Ok(relative)
}

fn copy_destination_relative(
    target: &crate::sample_sources::SampleSource,
    target_folder: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<PathBuf, String> {
    let base = if target_folder.as_os_str().is_empty() {
        PathBuf::from(file_name)
    } else {
        target_folder.join(file_name)
    };
    if !target.root.join(&base).exists() {
        return Ok(base);
    }
    let stem = Path::new(file_name)
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "sample".to_string());
    let extension = Path::new(file_name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());
    for index in 1..=999 {
        let suffix = format!("{stem}_copy{index:03}");
        let file_name = if let Some(ext) = &extension {
            format!("{suffix}.{ext}")
        } else {
            suffix
        };
        let candidate = if target_folder.as_os_str().is_empty() {
            PathBuf::from(&file_name)
        } else {
            target_folder.join(&file_name)
        };
        if !target.root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("Failed to find destination file name".into())
}
