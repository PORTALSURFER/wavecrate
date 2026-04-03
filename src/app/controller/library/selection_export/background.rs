//! Background selection-export worker implementation and timing capture.

use super::background_recording::{
    next_clip_export_path, record_clip_entry, record_crop_entry, record_slice_batch_entry,
};
use super::*;
use crate::app::controller::jobs::{
    SelectionClipDestination, SelectionClipExportSuccess, SelectionCropExportSuccess,
    SelectionExportAudioPayload, SelectionExportJob, SelectionExportMessage,
    SelectionExportPlaybackState, SelectionExportResult, SelectionExportSnapshot,
    SelectionExportTimings, SelectionSliceBatchExportSnapshot, SelectionSliceBatchExportSuccess,
};
use crate::app::controller::library::selection_edits::{
    apply_short_edge_fades_to_clip, next_crop_relative_path,
};
use crate::app::controller::playback::audio_samples::{crop_samples, decode_samples_from_bytes};
use std::borrow::Cow;
use std::path::Path;
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
    let prepare_started = Instant::now();
    let audio = resolve_selection_export_audio(&snapshot.audio)?;
    let mut prepared = prepare_selection_clip(&audio, &snapshot)?;
    let prepare = prepare_started.elapsed();

    let target_relative = next_clip_export_path(&snapshot, &destination);
    let (target_source_id, target_source_root) = match &destination {
        SelectionClipDestination::Folder {
            source_id,
            source_root,
            ..
        } => (source_id.clone(), source_root.clone()),
        SelectionClipDestination::Browser { .. } | SelectionClipDestination::ExternalDrag => {
            (snapshot.source_id.clone(), snapshot.source_root.clone())
        }
    };
    let absolute_path = target_source_root.join(&target_relative);
    let write_started = Instant::now();
    write_selection_clip(&absolute_path, &mut prepared, &snapshot)?;
    let write = write_started.elapsed();

    let register_started = Instant::now();
    let entry = record_clip_entry(&snapshot, &destination, target_relative.clone())?;
    let register = register_started.elapsed();
    let backup = crate::app::controller::undo::OverwriteBackup::capture_before(&absolute_path)?;
    backup.capture_after(&absolute_path)?;

    Ok(SelectionClipExportSuccess {
        request_id,
        source_id: target_source_id,
        source_root: target_source_root,
        entry,
        absolute_path,
        backup,
        destination,
        timings: SelectionExportTimings {
            prepare,
            write,
            register,
            total: started_at.elapsed(),
        },
    })
}

fn run_crop_export_job(
    request_id: u64,
    snapshot: SelectionExportSnapshot,
    playback: SelectionExportPlaybackState,
    started_at: Instant,
) -> Result<SelectionCropExportSuccess, String> {
    let prepare_started = Instant::now();
    let audio = resolve_selection_export_audio(&snapshot.audio)?;
    let mut prepared = prepare_selection_clip(&audio, &snapshot)?;
    let prepare = prepare_started.elapsed();

    let new_relative = next_crop_relative_path(&snapshot.relative_path, &snapshot.source_root)?;
    let absolute_path = snapshot.source_root.join(&new_relative);
    let write_started = Instant::now();
    write_selection_clip(&absolute_path, &mut prepared, &snapshot)?;
    let write = write_started.elapsed();

    let register_started = Instant::now();
    let entry = record_crop_entry(&snapshot, new_relative.clone())?;
    let register = register_started.elapsed();
    let backup = crate::app::controller::undo::OverwriteBackup::capture_before(&absolute_path)?;
    backup.capture_after(&absolute_path)?;

    Ok(SelectionCropExportSuccess {
        request_id,
        source_id: snapshot.source_id,
        source_root: snapshot.source_root,
        source_relative_path: snapshot.relative_path,
        entry,
        absolute_path,
        backup,
        tag: snapshot.target_tag.unwrap_or(Rating::NEUTRAL),
        playback,
        timings: SelectionExportTimings {
            prepare,
            write,
            register,
            total: started_at.elapsed(),
        },
    })
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
            Ok(entry) => entries.push(entry),
            Err(err) => errors.push(err),
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

struct PreparedSelectionClip {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

struct ResolvedSelectionExportAudio<'a> {
    samples: Cow<'a, [f32]>,
    sample_rate: u32,
    channels: u16,
}

fn resolve_selection_export_audio<'a>(
    audio: &'a SelectionExportAudioPayload,
) -> Result<ResolvedSelectionExportAudio<'a>, String> {
    match audio {
        SelectionExportAudioPayload::Decoded {
            samples,
            channels,
            sample_rate,
        } => Ok(ResolvedSelectionExportAudio {
            samples: Cow::Borrowed(samples.as_ref()),
            sample_rate: (*sample_rate).max(1),
            channels: (*channels).max(1),
        }),
        SelectionExportAudioPayload::Encoded { bytes } => {
            let decoded = decode_samples_from_bytes(bytes)?;
            Ok(ResolvedSelectionExportAudio {
                samples: Cow::Owned(decoded.samples),
                sample_rate: decoded.sample_rate.max(1),
                channels: decoded.channels.max(1),
            })
        }
    }
}

fn prepare_selection_clip(
    audio: &ResolvedSelectionExportAudio<'_>,
    snapshot: &SelectionExportSnapshot,
) -> Result<PreparedSelectionClip, String> {
    let (mut samples, sample_rate, channels) = (
        crop_samples(audio.samples.as_ref(), audio.channels, snapshot.bounds)?,
        audio.sample_rate,
        audio.channels,
    );
    if samples.is_empty() {
        return Err("Selection has no audio to export".to_string());
    }
    if snapshot.apply_edge_fades {
        let fade_duration =
            Duration::from_secs_f32(snapshot.edge_fade_ms.max(0.0).max(0.0) / 1000.0);
        apply_short_edge_fades_to_clip(&mut samples, channels as usize, sample_rate, fade_duration);
    }
    Ok(PreparedSelectionClip {
        samples,
        sample_rate,
        channels,
    })
}

fn write_selection_clip(
    absolute_path: &Path,
    prepared: &mut PreparedSelectionClip,
    _snapshot: &SelectionExportSnapshot,
) -> Result<(), String> {
    super::write_wav(
        absolute_path,
        &prepared.samples,
        prepared.sample_rate,
        prepared.channels,
    )
}

fn write_slice_batch_clip(
    absolute_path: &Path,
    samples: &[f32],
    snapshot: &SelectionSliceBatchExportSnapshot,
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let mut prepared = PreparedSelectionClip {
        samples: samples.to_vec(),
        sample_rate,
        channels,
    };
    if snapshot.apply_edge_fades {
        let fade_duration = Duration::from_secs_f32(snapshot.edge_fade_ms.max(0.0) / 1000.0);
        apply_short_edge_fades_to_clip(
            &mut prepared.samples,
            prepared.channels as usize,
            prepared.sample_rate,
            fade_duration,
        );
    }
    super::write_wav(
        absolute_path,
        &prepared.samples,
        prepared.sample_rate,
        prepared.channels,
    )
}
