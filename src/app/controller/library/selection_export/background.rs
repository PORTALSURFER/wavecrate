//! Background selection-export worker implementation and timing capture.

use super::background_recording::record_slice_batch_entry;
use super::helpers::cleanup_written_export_after_registration_failure;
use super::pipeline::{
    resolve_selection_export_audio, run_clip_export_pipeline, run_crop_export_pipeline,
    write_slice_batch_clip,
};
use super::*;
use crate::app::controller::jobs::{
    SelectionClipDestination, SelectionClipExportSuccess, SelectionCropExportSuccess,
    SelectionExportJob, SelectionExportMessage, SelectionExportPlaybackState,
    SelectionExportResult, SelectionExportSnapshot, SelectionSliceBatchExportSnapshot,
    SelectionSliceBatchExportSuccess,
};
use crate::app::controller::playback::audio_samples::crop_samples;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// Run one background selection-export job and return its typed completion payload.
pub(crate) fn run_selection_export_job(job: SelectionExportJob) -> SelectionExportResult {
    let started_at = Instant::now();
    match job {
        SelectionExportJob::Clip {
            request_id,
            snapshot,
            destination,
        } => SelectionExportResult::Clip {
            request_id,
            result: run_clip_export_job(request_id, snapshot, destination, started_at),
        },
        SelectionExportJob::CropNewSample {
            request_id,
            snapshot,
            playback,
        } => SelectionExportResult::CropNewSample {
            request_id,
            result: run_crop_export_job(request_id, snapshot, playback, started_at),
        },
        SelectionExportJob::SliceBatch {
            request_id,
            snapshot,
        } => SelectionExportResult::SliceBatch {
            request_id,
            result: run_slice_batch_export(request_id, snapshot, None, started_at),
        },
    }
}

/// Run one streamed slice-batch export job and forward progress to the caller.
pub(crate) fn run_slice_batch_export_job(
    job: SelectionExportJob,
    sender: &Sender<SelectionExportMessage>,
) {
    let started_at = Instant::now();
    let result = match job {
        SelectionExportJob::SliceBatch {
            request_id,
            snapshot,
        } => SelectionExportResult::SliceBatch {
            request_id,
            result: run_slice_batch_export(request_id, snapshot, Some(sender), started_at),
        },
        other => run_selection_export_job(other),
    };
    let _ = sender.send(SelectionExportMessage::Finished(result));
}

fn run_clip_export_job(
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    destination: SelectionClipDestination,
    started_at: Instant,
) -> Result<SelectionClipExportSuccess, String> {
    run_clip_export_pipeline(request_id, snapshot, destination, started_at)
}

fn run_crop_export_job(
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    playback: SelectionExportPlaybackState,
    started_at: Instant,
) -> Result<SelectionCropExportSuccess, String> {
    run_crop_export_pipeline(request_id, snapshot, playback, started_at)
}

fn run_slice_batch_export(
    request_id: u64,
    snapshot: SelectionSliceBatchExportSnapshot,
    sender: Option<&Sender<SelectionExportMessage>>,
    started_at: Instant,
) -> Result<SelectionSliceBatchExportSuccess, String> {
    let prepare_started = Instant::now();
    let audio = resolve_selection_export_audio(&snapshot.audio)?;
    let prepare = prepare_started.elapsed();
    let mut counter = 1usize;
    let mut entries = Vec::with_capacity(snapshot.slices.len());
    let mut exported_slices = Vec::with_capacity(snapshot.slices.len());
    let mut errors = Vec::new();
    let mut write = Duration::default();
    let mut register = Duration::default();

    for (index, slice) in snapshot.slices.iter().copied().enumerate() {
        let target_relative = super::slice_batch::next_slice_path_in_dir_for_root(
            &snapshot.source_root,
            &snapshot.relative_path,
            snapshot.profile,
            &mut counter,
        );
        let absolute_path = snapshot.source_root.join(&target_relative);
        let detail = Some(format!("Saving {}", target_relative.display()));

        let write_started = Instant::now();
        let write_result =
            crop_samples(audio.samples.as_ref(), audio.channels, slice).and_then(|samples| {
                write_slice_batch_clip(
                    &absolute_path,
                    &samples,
                    &snapshot,
                    audio.sample_rate,
                    audio.channels,
                )
            });
        write += write_started.elapsed();
        if let Err(err) = write_result {
            errors.push(format!(
                "Failed to export {}: {err}",
                target_relative.display()
            ));
            if let Some(progress) = sender {
                let _ = progress.send(SelectionExportMessage::Progress {
                    request_id,
                    total: snapshot.slices.len(),
                    completed: index + 1,
                    detail,
                });
            }
            continue;
        }

        let register_started = Instant::now();
        match record_slice_batch_entry(&snapshot, target_relative) {
            Ok(entry) => {
                entries.push(entry);
                exported_slices.push(slice);
            }
            Err(err) => {
                errors.push(cleanup_written_export_after_registration_failure(
                    &absolute_path,
                    err,
                ));
            }
        }
        register += register_started.elapsed();

        if let Some(progress) = sender {
            let _ = progress.send(SelectionExportMessage::Progress {
                request_id,
                total: snapshot.slices.len(),
                completed: index + 1,
                detail,
            });
        }
    }

    Ok(SelectionSliceBatchExportSuccess {
        request_id,
        source_id: snapshot.source_id,
        source_root: snapshot.source_root,
        source_relative_path: snapshot.relative_path,
        source_slices: exported_slices,
        source_duration_seconds: snapshot.source_duration_seconds,
        entries,
        errors,
        timings: SelectionExportTimings {
            prepare,
            write,
            register,
            total: started_at.elapsed(),
        },
    })
}
