//! Background selection-export worker implementation and timing capture.

use super::helpers::fast_content_hash;
use super::*;
use crate::app::controller::jobs::{
    SelectionClipDestination, SelectionClipExportSuccess, SelectionCropExportSuccess,
    SelectionExportAudioPayload, SelectionExportJob, SelectionExportMessage,
    SelectionExportPlaybackState, SelectionExportResult, SelectionExportSnapshot,
    SelectionExportTimings, SelectionSliceBatchExportSnapshot, SelectionSliceBatchExportSuccess,
};
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::library::selection_edits::{
    apply_short_edge_fades_to_clip, next_crop_relative_path,
};
use crate::app::controller::playback::audio_samples::{crop_samples, decode_samples_from_bytes};
use crate::sample_sources::{Rating, SampleSource, SourceDatabase, WavEntry};
use rusqlite::params;
use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant, SystemTime};

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

    let target_relative = match &destination {
        SelectionClipDestination::Browser {
            folder_override: Some(folder),
            ..
        }
        | SelectionClipDestination::Folder { folder, .. } => next_selection_path_in_dir(
            &snapshot.source_root,
            &folder.join(file_name_hint(&snapshot)),
        ),
        SelectionClipDestination::Browser { .. } | SelectionClipDestination::ExternalDrag => {
            next_selection_path_in_dir(&snapshot.source_root, &snapshot.relative_path)
        }
    };
    let absolute_path = snapshot.source_root.join(&target_relative);
    let write_started = Instant::now();
    write_selection_clip(&absolute_path, &mut prepared, &snapshot)?;
    let write = write_started.elapsed();

    let register_started = Instant::now();
    let entry = record_clip_entry(&snapshot, target_relative.clone())?;
    let register = register_started.elapsed();

    Ok(SelectionClipExportSuccess {
        request_id,
        source_id: snapshot.source_id,
        source_root: snapshot.source_root,
        entry,
        absolute_path,
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

    Ok(SelectionCropExportSuccess {
        request_id,
        source_id: snapshot.source_id,
        source_root: snapshot.source_root,
        source_relative_path: snapshot.relative_path,
        entry,
        absolute_path,
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

fn record_clip_entry(
    snapshot: &SelectionExportSnapshot,
    relative_path: PathBuf,
) -> Result<WavEntry, String> {
    let entry = build_written_entry(
        &snapshot.source_root,
        relative_path,
        snapshot.target_tag.unwrap_or(Rating::NEUTRAL),
        snapshot.looped,
    )?;
    let source = sample_source(snapshot);
    let db = SourceDatabase::open_fast(&source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
        .map_err(|err| format!("Failed to register clip: {err}"))?;
    if entry.tag != Rating::NEUTRAL {
        db.set_tag(&entry.relative_path, entry.tag)
            .map_err(|err| format!("Failed to tag clip: {err}"))?;
    }
    if entry.looped {
        db.set_looped(&entry.relative_path, true)
            .map_err(|err| format!("Failed to mark clip as looped: {err}"))?;
    }
    if let Some(bpm) = snapshot.bpm.filter(|_| entry.looped) {
        persist_selection_bpm(&source, &entry, bpm)?;
    }
    Ok(entry)
}

fn record_crop_entry(
    snapshot: &SelectionExportSnapshot,
    relative_path: PathBuf,
) -> Result<WavEntry, String> {
    let mut entry = build_written_entry(
        &snapshot.source_root,
        relative_path,
        snapshot.target_tag.unwrap_or(Rating::NEUTRAL),
        false,
    )?;
    entry.looped = false;
    entry.tag = snapshot.target_tag.unwrap_or(Rating::NEUTRAL);
    let source = sample_source(snapshot);
    let db = SourceDatabase::open_fast(&source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    db.set_tag(&entry.relative_path, entry.tag)
        .map_err(|err| format!("Failed to sync tag: {err}"))?;
    Ok(entry)
}

fn record_slice_batch_entry(
    snapshot: &SelectionSliceBatchExportSnapshot,
    relative_path: PathBuf,
) -> Result<WavEntry, String> {
    let entry = build_written_entry(&snapshot.source_root, relative_path, Rating::NEUTRAL, false)?;
    let source = SampleSource {
        id: snapshot.source_id.clone(),
        root: snapshot.source_root.clone(),
    };
    let db = SourceDatabase::open_fast(&source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
        .map_err(|err| format!("Failed to register slice: {err}"))?;
    Ok(entry)
}

fn build_written_entry(
    source_root: &Path,
    relative_path: PathBuf,
    tag: Rating,
    looped: bool,
) -> Result<WavEntry, String> {
    let metadata = fs::metadata(source_root.join(&relative_path))
        .map_err(|err| format!("Failed to read saved clip: {err}"))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for clip: {err}"))?
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| "Clip modified time is before epoch".to_string())?
        .as_nanos() as i64;
    Ok(WavEntry {
        relative_path,
        file_size: metadata.len(),
        modified_ns,
        content_hash: None,
        tag,
        looped,
        locked: false,
        missing: false,
        last_played_at: None,
    })
}

fn persist_selection_bpm(source: &SampleSource, entry: &WavEntry, bpm: f32) -> Result<(), String> {
    if !bpm.is_finite() || bpm <= 0.0 {
        return Ok(());
    }
    let size = i64::try_from(entry.file_size)
        .map_err(|_| "Clip size exceeds database limits".to_string())?;
    let content_hash = fast_content_hash(entry.file_size, entry.modified_ns);
    let conn = analysis_jobs::open_source_db(&source.root)
        .map_err(|err| format!("Failed to open analysis database: {err}"))?;
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), &entry.relative_path);
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version, bpm)
         VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, ?5)
         ON CONFLICT(sample_id) DO UPDATE SET
             content_hash = excluded.content_hash,
             size = excluded.size,
             mtime_ns = excluded.mtime_ns,
             bpm = excluded.bpm",
        params![
            sample_id,
            content_hash,
            size,
            entry.modified_ns,
            bpm as f64
        ],
    )
    .map_err(|err| format!("Failed to store clip BPM: {err}"))?;
    Ok(())
}

fn sample_source(snapshot: &SelectionExportSnapshot) -> SampleSource {
    SampleSource {
        id: snapshot.source_id.clone(),
        root: snapshot.source_root.clone(),
    }
}

fn file_name_hint(snapshot: &SelectionExportSnapshot) -> PathBuf {
    snapshot
        .relative_path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("selection.wav"))
}

fn next_selection_path_in_dir(root: &Path, original: &Path) -> PathBuf {
    let parent = original.parent().unwrap_or_else(|| Path::new(""));
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("selection");
    let stem = AppController::strip_selection_suffix(stem);
    let mut counter = 1u32;
    loop {
        let candidate = parent.join(format!("{stem}_selection_{counter:03}.wav"));
        if !root.join(&candidate).exists() {
            return candidate;
        }
        counter += 1;
    }
}
