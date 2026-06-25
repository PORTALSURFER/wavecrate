use super::*;
use crate::app::controller::jobs::WaveformSlideCommitResult;
use crate::app::controller::library::wav_io::file_metadata;
use hound::SampleFormat;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub(super) fn run_waveform_slide_job(
    state: WaveformSlideState,
    rotated: Vec<f32>,
    cancel: Arc<AtomicBool>,
) -> WaveformSlideCommitResult {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return WaveformSlideCommitResult {
            source_id: state.source.id,
            relative_path: state.relative_path,
            absolute_path: state.absolute_path,
            entry: None,
            backup: None,
            result: Err(String::from("Circular slide cancelled")),
        };
    }
    let result = commit_waveform_slide(&state, &rotated);
    match result {
        Ok((entry, backup)) => WaveformSlideCommitResult {
            source_id: state.source.id,
            relative_path: state.relative_path,
            absolute_path: state.absolute_path,
            entry: Some(entry),
            backup: Some(backup),
            result: Ok(()),
        },
        Err(err) => WaveformSlideCommitResult {
            source_id: state.source.id,
            relative_path: state.relative_path,
            absolute_path: state.absolute_path,
            entry: None,
            backup: None,
            result: Err(err),
        },
    }
}

fn commit_waveform_slide(
    state: &WaveformSlideState,
    rotated: &[f32],
) -> Result<(WavEntry, crate::app::controller::undo::OverwriteBackup), String> {
    let backup =
        crate::app::controller::undo::OverwriteBackup::capture_before(&state.absolute_path)?;
    let spec = slide_wav_spec(state.spec_channels, state.sample_rate);
    write_waveform_wav(&state.absolute_path, rotated, spec)?;
    let (file_size, modified_ns) = file_metadata(&state.absolute_path)?;
    let db = crate::sample_sources::SourceDatabase::open(&state.source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let tag = db
        .tag_for_path(&state.relative_path)
        .map_err(|err| format!("Failed to read tag: {err}"))?
        .ok_or_else(|| "Sample not found in database".to_string())?;
    db.upsert_file(&state.relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    db.set_tag(&state.relative_path, tag)
        .map_err(|err| format!("Failed to sync tag: {err}"))?;
    let last_played_at = db
        .last_played_at_for_path(&state.relative_path)
        .map_err(|err| format!("Failed to read playback age: {err}"))?;
    let looped = db
        .looped_for_path(&state.relative_path)
        .map_err(|err| format!("Failed to read loop marker: {err}"))?
        .unwrap_or(false);
    let locked = db
        .locked_for_path(&state.relative_path)
        .map_err(|err| format!("Failed to read lock state: {err}"))?
        .unwrap_or(false);
    backup.capture_after(&state.absolute_path)?;
    Ok((
        WavEntry {
            relative_path: state.relative_path.clone(),
            file_size,
            modified_ns,
            content_hash: None,
            tag,
            looped,
            sound_type: None,
            locked,
            missing: false,
            last_played_at,
            last_curated_at: None,
            user_tag: None,
            tag_named: false,
            normal_tags: Vec::new(),
        },
        backup,
    ))
}

pub(super) fn slide_wav_spec(channels: u16, sample_rate: u32) -> hound::WavSpec {
    hound::WavSpec {
        channels,
        sample_rate: sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    }
}

pub(super) fn write_waveform_wav(
    target: &PathBuf,
    samples: &[f32],
    spec: hound::WavSpec,
) -> Result<(), String> {
    let mut writer = hound::WavWriter::create(target, spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|err| format!("Failed to write sample: {err}"))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize wav: {err}"))
}
