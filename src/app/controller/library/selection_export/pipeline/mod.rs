//! Staged worker-side selection export pipelines.

mod audio;

use super::background_recording::{next_clip_export_path, record_clip_entry, record_crop_entry};
use super::helpers::cleanup_written_export_after_registration_failure;
use super::*;
use crate::app::controller::jobs::{
    SelectionClipDestination, SelectionClipExportSuccess, SelectionCropExportSuccess,
    SelectionExportPlaybackState, SelectionExportSnapshot, SelectionExportTimings,
};
use crate::app::controller::library::selection_edits::next_crop_relative_path;
use audio::{prepare_selection_clip, write_selection_clip};
pub(super) use audio::{resolve_selection_export_audio, write_slice_batch_clip};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Run the staged pipeline for one selection clip export.
pub(super) fn run_clip_export_pipeline(
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    destination: SelectionClipDestination,
    started_at: Instant,
) -> Result<SelectionClipExportSuccess, String> {
    SelectionClipExportPipeline {
        request_id,
        snapshot,
        destination,
    }
    .run(started_at)
}

/// Run the staged pipeline for one crop-to-new-sample export.
pub(super) fn run_crop_export_pipeline(
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    playback: SelectionExportPlaybackState,
    started_at: Instant,
) -> Result<SelectionCropExportSuccess, String> {
    SelectionCropExportPipeline {
        request_id,
        snapshot,
        playback,
    }
    .run(started_at)
}

struct SelectionClipExportPipeline {
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    destination: SelectionClipDestination,
}

impl SelectionClipExportPipeline {
    fn run(self, started_at: Instant) -> Result<SelectionClipExportSuccess, String> {
        let prepare_started = Instant::now();
        let destination = self.plan_destination()?;
        let audio = resolve_selection_export_audio(&self.snapshot.audio)?;
        let mut prepared = prepare_selection_clip(&audio, &self.snapshot)?;
        let prepare = prepare_started.elapsed();

        let write_started = Instant::now();
        write_selection_clip(&destination.absolute_path, &mut prepared, &self.snapshot)?;
        let write = write_started.elapsed();

        let register_started = Instant::now();
        let entry = record_clip_entry(
            &self.snapshot,
            &self.destination,
            destination.relative_path.clone(),
        )
        .map_err(|err| {
            cleanup_written_export_after_registration_failure(&destination.absolute_path, err)
        })?;
        let register = register_started.elapsed();
        let backup = crate::app::controller::undo::OverwriteBackup::capture_before(
            &destination.absolute_path,
        )?;
        backup.capture_after(&destination.absolute_path)?;

        Ok(SelectionClipExportSuccess {
            request_id: self.request_id,
            source_id: destination.source_id,
            source_root: destination.source_root,
            entry,
            origin_source_id: self.snapshot.source_id,
            origin_source_root: self.snapshot.source_root,
            origin_relative_path: self.snapshot.relative_path,
            origin_bounds: self.snapshot.bounds,
            origin_duration_seconds: self.snapshot.source_duration_seconds,
            absolute_path: destination.absolute_path,
            backup,
            destination: self.destination,
            timings: SelectionExportTimings {
                prepare,
                write,
                register,
                total: started_at.elapsed(),
            },
        })
    }

    fn plan_destination(&self) -> Result<SelectionClipDestinationPlan, String> {
        let relative_path = next_clip_export_path(&self.snapshot, &self.destination);
        let (source_id, source_root) = match &self.destination {
            SelectionClipDestination::Folder {
                source_id,
                source_root,
                folder,
                ..
            } => {
                require_existing_dir(source_root, "Export source root")?;
                require_existing_dir(&source_root.join(folder), "Folder")?;
                (source_id.clone(), source_root.clone())
            }
            SelectionClipDestination::Browser {
                folder_override: Some(folder),
                ..
            } => {
                require_existing_dir(&self.snapshot.source_root, "Export source root")?;
                require_existing_dir(&self.snapshot.source_root.join(folder), "Folder")?;
                (
                    self.snapshot.source_id.clone(),
                    self.snapshot.source_root.clone(),
                )
            }
            SelectionClipDestination::Browser { .. } | SelectionClipDestination::ExternalDrag => {
                require_existing_dir(&self.snapshot.source_root, "Export source root")?;
                (
                    self.snapshot.source_id.clone(),
                    self.snapshot.source_root.clone(),
                )
            }
        };
        let absolute_path = source_root.join(&relative_path);
        if absolute_path.exists() {
            return Err(format!(
                "Export target already exists: {}",
                relative_path.display()
            ));
        }
        Ok(SelectionClipDestinationPlan {
            source_id,
            source_root,
            relative_path,
            absolute_path,
        })
    }
}

struct SelectionCropExportPipeline {
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    playback: SelectionExportPlaybackState,
}

impl SelectionCropExportPipeline {
    fn run(self, started_at: Instant) -> Result<SelectionCropExportSuccess, String> {
        let prepare_started = Instant::now();
        require_existing_dir(&self.snapshot.source_root, "Export source root")?;
        let audio = resolve_selection_export_audio(&self.snapshot.audio)?;
        let mut prepared = prepare_selection_clip(&audio, &self.snapshot)?;
        let prepare = prepare_started.elapsed();

        let new_relative =
            next_crop_relative_path(&self.snapshot.relative_path, &self.snapshot.source_root)?;
        let absolute_path = self.snapshot.source_root.join(&new_relative);
        if absolute_path.exists() {
            return Err(format!(
                "Export target already exists: {}",
                new_relative.display()
            ));
        }

        let write_started = Instant::now();
        write_selection_clip(&absolute_path, &mut prepared, &self.snapshot)?;
        let write = write_started.elapsed();

        let register_started = Instant::now();
        let entry = record_crop_entry(&self.snapshot, new_relative.clone()).map_err(|err| {
            cleanup_written_export_after_registration_failure(&absolute_path, err)
        })?;
        let register = register_started.elapsed();
        let backup = crate::app::controller::undo::OverwriteBackup::capture_before(&absolute_path)?;
        backup.capture_after(&absolute_path)?;

        Ok(SelectionCropExportSuccess {
            request_id: self.request_id,
            source_id: self.snapshot.source_id,
            source_root: self.snapshot.source_root,
            source_relative_path: self.snapshot.relative_path,
            source_bounds: self.snapshot.bounds,
            source_duration_seconds: self.snapshot.source_duration_seconds,
            entry,
            absolute_path,
            backup,
            tag: self.snapshot.target_tag.unwrap_or(Rating::NEUTRAL),
            playback: self.playback,
            timings: SelectionExportTimings {
                prepare,
                write,
                register,
                total: started_at.elapsed(),
            },
        })
    }
}

struct SelectionClipDestinationPlan {
    source_id: SourceId,
    source_root: PathBuf,
    relative_path: PathBuf,
    absolute_path: PathBuf,
}

fn require_existing_dir(path: &Path, label: &str) -> Result<(), String> {
    path.is_dir()
        .then_some(())
        .ok_or_else(|| format!("{label} not found: {}", path.display()))
}
