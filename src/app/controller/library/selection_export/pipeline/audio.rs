//! Audio preparation and writing helpers for selection export pipelines.

use crate::app::controller::jobs::{
    SelectionExportAudioPayload, SelectionExportSnapshot, SelectionSliceBatchExportSnapshot,
};
use crate::app::controller::library::selection_edits::apply_short_edge_fades_to_clip;
use crate::app::controller::playback::audio_samples::{crop_samples, decode_samples_from_bytes};
use std::borrow::Cow;
use std::path::Path;
use std::time::Duration;

pub(super) struct PreparedSelectionClip {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

pub(in crate::app::controller::library::selection_export) struct ResolvedSelectionExportAudio<'a> {
    pub(in crate::app::controller::library::selection_export) samples: Cow<'a, [f32]>,
    pub(in crate::app::controller::library::selection_export) sample_rate: u32,
    pub(in crate::app::controller::library::selection_export) channels: u16,
}

pub(in crate::app::controller::library::selection_export) fn resolve_selection_export_audio<'a>(
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

pub(super) fn prepare_selection_clip(
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

pub(super) fn write_selection_clip(
    absolute_path: &Path,
    prepared: &mut PreparedSelectionClip,
    snapshot: &SelectionExportSnapshot,
) -> Result<(), String> {
    crate::app::controller::playback::audio_samples::write_wav_with_spec(
        absolute_path,
        &prepared.samples,
        snapshot
            .write_format
            .wav_spec_for_source(prepared.channels, prepared.sample_rate),
    )
}

pub(in crate::app::controller::library::selection_export) fn write_slice_batch_clip(
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
    crate::app::controller::playback::audio_samples::write_wav_with_spec(
        absolute_path,
        &prepared.samples,
        snapshot
            .write_format
            .wav_spec_for_source(prepared.channels, prepared.sample_rate),
    )
}
