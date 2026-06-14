use super::super::*;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;
use std::borrow::Cow;

pub(super) struct DuplicateDetectionInput<'a> {
    pub(super) samples: Cow<'a, [f32]>,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) total_frames: usize,
    pub(super) scan: DuplicateWindowScanConfig,
    pub(super) transient_frames: Vec<usize>,
}

#[derive(Clone, Copy)]
pub(super) struct DuplicateWindowScanConfig {
    pub(super) anchor_start_frame: usize,
    pub(super) window_frames: usize,
}

pub(super) fn duplicate_detection_input(
    controller: &AppController,
) -> Result<DuplicateDetectionInput<'_>, String> {
    let (samples, sample_rate, channels) = waveform_slice_analysis_audio(controller)?;
    let total_frames = samples.len() / channels.max(1) as usize;
    if total_frames == 0 {
        return Err("No audio data to scan".to_string());
    }
    let scan = current_duplicate_window_scan_config(controller, total_frames).ok_or_else(|| {
        "Create a playback selection to define the duplicate window size".to_string()
    })?;
    let transient_frames = current_duplicate_candidate_event_frames(controller, total_frames);
    Ok(DuplicateDetectionInput {
        samples,
        sample_rate,
        channels,
        total_frames,
        scan,
        transient_frames,
    })
}

fn current_duplicate_window_scan_config(
    controller: &AppController,
    total_frames: usize,
) -> Option<DuplicateWindowScanConfig> {
    let selection = controller.ui.waveform.selection?;
    let (anchor_start_frame, anchor_end_frame) = selection_frame_bounds(total_frames, selection);
    let window_frames = anchor_end_frame.saturating_sub(anchor_start_frame);
    (window_frames > 0).then_some(DuplicateWindowScanConfig {
        anchor_start_frame,
        window_frames,
    })
}

fn current_duplicate_candidate_event_frames(
    controller: &AppController,
    total_frames: usize,
) -> Vec<usize> {
    let mut frames = controller
        .ui
        .waveform
        .transients
        .iter()
        .copied()
        .map(|value| {
            ((value.clamp(0.0, 1.0) * total_frames as f32).round() as usize).min(total_frames)
        })
        .filter(|frame| *frame < total_frames)
        .collect::<Vec<_>>();
    frames.sort_unstable();
    frames.dedup();
    frames
}

fn waveform_slice_analysis_audio(
    controller: &AppController,
) -> Result<(Cow<'_, [f32]>, u32, u16), String> {
    let audio = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .ok_or_else(|| "Load a sample before slicing".to_string())?;
    if let Some(decoded) = controller.sample_view.waveform.decoded.as_ref()
        && decoded.peaks.is_none()
        && !decoded.samples.is_empty()
    {
        return Ok((
            Cow::Borrowed(decoded.samples.as_ref()),
            decoded.sample_rate.max(1),
            decoded.channels.max(1),
        ));
    }
    let decoded = decode_samples_from_bytes(&audio.bytes)?;
    Ok((
        Cow::Owned(decoded.samples),
        decoded.sample_rate.max(1),
        decoded.channels.max(1),
    ))
}

fn selection_frame_bounds(total_frames: usize, bounds: SelectionRange) -> (usize, usize) {
    let start_frame = ((bounds.start() * total_frames as f32).floor() as usize)
        .min(total_frames.saturating_sub(1));
    let mut end_frame = ((bounds.end() * total_frames as f32).ceil() as usize).min(total_frames);
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    (start_frame, end_frame)
}
