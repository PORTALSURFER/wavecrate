use std::{fs, path::PathBuf, sync::Arc};

use wavecrate::selection::SelectionRange;

use super::{
    WaveformState,
    audio_file::{PersistedPlaybackCacheFile, is_wav_path, read_wav_playback_samples},
};

const PROFILE_BINS: usize = 192;
const MAX_SCAN_WINDOWS: usize = 8_000;
const MIN_ANCHOR_FRAMES: usize = 64;
const SIMILAR_SECTION_THRESHOLD: f32 = 0.86;
const MIN_PROFILE_RMS: f32 = 1.0e-4;

#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::native_app::waveform) struct SimilarSectionsState {
    enabled: bool,
    anchor: Option<SelectionRange>,
    ranges: Vec<SelectionRange>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SimilarSectionsRequest {
    path: PathBuf,
    content_revision: u64,
    source: SimilarSectionsSource,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    anchor: SelectionRange,
}

#[derive(Clone, Debug)]
enum SimilarSectionsSource {
    InterleavedF32Samples(Arc<[f32]>),
    InterleavedF32File(PersistedPlaybackCacheFile),
    WavBytes(Arc<[u8]>),
    WavFile,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SimilarSectionsResult {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) content_revision: u64,
    pub(in crate::native_app) anchor: SelectionRange,
    pub(in crate::native_app) result: Result<SimilarSectionsPayload, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SimilarSectionsPayload {
    pub(in crate::native_app) ranges: Vec<SelectionRange>,
}

#[derive(Clone, Debug)]
struct SimilarityProfile {
    signed: Vec<f32>,
    energy: Vec<f32>,
    rms: f32,
}

#[derive(Clone, Copy, Debug)]
struct CandidateMatch {
    start_frame: usize,
    score: f32,
}

impl SimilarSectionsRequest {
    pub(in crate::native_app) fn anchor(&self) -> SelectionRange {
        self.anchor
    }
}

impl WaveformState {
    pub(in crate::native_app) fn similar_sections_request(
        &self,
    ) -> Result<SimilarSectionsRequest, String> {
        let anchor = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
            .ok_or_else(|| String::from("Set a playmark selection first"))?;
        let source = self.similar_sections_source()?;
        Ok(SimilarSectionsRequest {
            path: self.file.path.clone(),
            content_revision: self.file.content_revision(),
            source,
            sample_rate: self.file.sample_rate,
            channels: self.file.channels,
            frames: self.file.frames,
            anchor,
        })
    }

    pub(in crate::native_app) fn similar_sections_enabled(&self) -> bool {
        self.similar_sections.enabled
    }

    pub(in crate::native_app) fn similar_section_ranges(&self) -> &[SelectionRange] {
        &self.similar_sections.ranges
    }

    pub(in crate::native_app) fn start_similar_sections(&mut self, anchor: SelectionRange) {
        self.similar_sections = SimilarSectionsState {
            enabled: true,
            anchor: Some(anchor),
            ranges: Vec::new(),
        };
    }

    pub(in crate::native_app) fn clear_similar_sections(&mut self) {
        self.similar_sections = SimilarSectionsState::default();
    }

    pub(in crate::native_app) fn similar_sections_result_applies(
        &self,
        result: &SimilarSectionsResult,
    ) -> bool {
        self.similar_sections.enabled
            && self.similar_sections.anchor == Some(result.anchor)
            && self.file.path == result.path
            && self.file.content_revision() == result.content_revision
    }

    pub(in crate::native_app) fn finish_similar_sections_scan(
        &mut self,
        ranges: Vec<SelectionRange>,
    ) {
        self.similar_sections.ranges = ranges;
    }

    fn similar_sections_source(&self) -> Result<SimilarSectionsSource, String> {
        if let Some(samples) = self.file.playback_samples.as_ref() {
            return Ok(SimilarSectionsSource::InterleavedF32Samples(Arc::clone(
                samples,
            )));
        }
        if let Some(cache_file) = self.file.playback_cache_file.as_ref() {
            return Ok(SimilarSectionsSource::InterleavedF32File(
                cache_file.clone(),
            ));
        }
        if !self.file.audio_bytes.is_empty() && is_wav_path(&self.file.path) {
            return Ok(SimilarSectionsSource::WavBytes(Arc::clone(
                &self.file.audio_bytes,
            )));
        }
        if self.file.has_loaded_sample_metadata() && is_wav_path(&self.file.path) {
            return Ok(SimilarSectionsSource::WavFile);
        }
        Err(String::from(
            "Similar section scan needs a WAV file or decoded playback cache",
        ))
    }
}

pub(in crate::native_app) fn execute_similar_sections_scan(
    request: SimilarSectionsRequest,
) -> SimilarSectionsResult {
    SimilarSectionsResult {
        path: request.path.clone(),
        content_revision: request.content_revision,
        anchor: request.anchor,
        result: scan_similar_sections(&request),
    }
}

fn scan_similar_sections(
    request: &SimilarSectionsRequest,
) -> Result<SimilarSectionsPayload, String> {
    let samples = request.source.load_samples(&request.path)?;
    validate_request(request, samples.len())?;
    let anchor_bounds = request.anchor.frame_bounds(request.frames);
    let window_frames = anchor_bounds
        .end_frame
        .saturating_sub(anchor_bounds.start_frame);
    if window_frames < MIN_ANCHOR_FRAMES {
        return Err(String::from("Select a slightly longer playmark section"));
    }

    let anchor_profile = build_profile(
        &samples,
        request.channels,
        request.frames,
        anchor_bounds.start_frame,
        window_frames,
    )
    .ok_or_else(|| String::from("Selected playmark is too quiet to compare"))?;
    let hop = scan_hop_frames(request.frames, window_frames, request.sample_rate);
    let candidates = collect_candidate_matches(
        request,
        &samples,
        &anchor_profile,
        anchor_bounds.start_frame,
        anchor_bounds.end_frame,
        window_frames,
        hop,
    );
    let ranges = group_candidate_matches(candidates, request.frames, window_frames, hop);
    Ok(SimilarSectionsPayload { ranges })
}

fn validate_request(request: &SimilarSectionsRequest, sample_count: usize) -> Result<(), String> {
    if request.frames == 0 || request.channels == 0 {
        return Err(String::from("Loaded sample has no decoded audio frames"));
    }
    let expected_samples = request.frames.saturating_mul(request.channels);
    if sample_count < expected_samples {
        return Err(String::from("Decoded playback data is incomplete"));
    }
    Ok(())
}

impl SimilarSectionsSource {
    fn load_samples(&self, path: &std::path::Path) -> Result<Arc<[f32]>, String> {
        match self {
            Self::InterleavedF32Samples(samples) => Ok(Arc::clone(samples)),
            Self::InterleavedF32File(cache_file) => read_interleaved_f32_file(cache_file),
            Self::WavBytes(bytes) => read_wav_playback_samples(bytes).map(Arc::from),
            Self::WavFile => {
                let bytes: Arc<[u8]> = fs::read(path).map(Arc::from).map_err(|err| {
                    format!("failed to read source WAV {}: {err}", path.display())
                })?;
                read_wav_playback_samples(&bytes).map(Arc::from)
            }
        }
    }
}

fn read_interleaved_f32_file(
    cache_file: &PersistedPlaybackCacheFile,
) -> Result<Arc<[f32]>, String> {
    let bytes = fs::read(&cache_file.path).map_err(|err| {
        format!(
            "failed to read playback cache {}: {err}",
            cache_file.path.display()
        )
    })?;
    let expected_bytes = cache_file
        .sample_count
        .checked_mul(std::mem::size_of::<f32>() as u64)
        .ok_or_else(|| String::from("Playback cache is too large"))?;
    if bytes.len() as u64 != expected_bytes {
        return Err(String::from(
            "Playback cache size does not match its metadata",
        ));
    }
    let mut samples = Vec::with_capacity(cache_file.sample_count as usize);
    for chunk in bytes.chunks_exact(std::mem::size_of::<f32>()) {
        samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(Arc::from(samples))
}

fn scan_hop_frames(frames: usize, window_frames: usize, sample_rate: u32) -> usize {
    let relative_hop = (window_frames / 128).max(1);
    let time_hop = (sample_rate as usize / 250).max(1);
    let mut hop = relative_hop.min(time_hop).max(1);
    let scan_span = frames.saturating_sub(window_frames);
    let candidate_count = scan_span / hop + 1;
    if candidate_count > MAX_SCAN_WINDOWS {
        hop = (scan_span / MAX_SCAN_WINDOWS).max(hop).max(1);
    }
    hop
}

fn collect_candidate_matches(
    request: &SimilarSectionsRequest,
    samples: &[f32],
    anchor_profile: &SimilarityProfile,
    anchor_start: usize,
    anchor_end: usize,
    window_frames: usize,
    hop: usize,
) -> Vec<CandidateMatch> {
    let mut candidates = Vec::new();
    let max_start = request.frames.saturating_sub(window_frames);
    let mut start_frame = 0;
    while start_frame <= max_start {
        let end_frame = start_frame + window_frames;
        if !ranges_overlap(start_frame, end_frame, anchor_start, anchor_end)
            && let Some(profile) = build_profile(
                samples,
                request.channels,
                request.frames,
                start_frame,
                window_frames,
            )
        {
            let score = profile_similarity(anchor_profile, &profile);
            if score >= SIMILAR_SECTION_THRESHOLD {
                candidates.push(CandidateMatch { start_frame, score });
            }
        }
        start_frame = start_frame.saturating_add(hop);
        if hop == 0 {
            break;
        }
    }
    candidates
}

fn group_candidate_matches(
    mut candidates: Vec<CandidateMatch>,
    frames: usize,
    window_frames: usize,
    hop: usize,
) -> Vec<SelectionRange> {
    candidates.sort_by_key(|candidate| candidate.start_frame);
    let mut ranges = Vec::new();
    let mut group: Option<CandidateMatch> = None;
    let mut group_end = 0usize;

    for candidate in candidates {
        let candidate_end = candidate.start_frame.saturating_add(window_frames);
        if group.is_some() && candidate.start_frame <= group_end.saturating_add(hop) {
            if let Some(best) = group.as_mut()
                && candidate.score > best.score
            {
                *best = candidate;
            }
            group_end = group_end.max(candidate_end);
            continue;
        }

        if let Some(best) = group.take() {
            ranges.push(range_for_match(best, frames, window_frames));
        }
        group = Some(candidate);
        group_end = candidate_end;
    }

    if let Some(best) = group {
        ranges.push(range_for_match(best, frames, window_frames));
    }
    ranges
}

fn range_for_match(match_: CandidateMatch, frames: usize, window_frames: usize) -> SelectionRange {
    SelectionRange::from_frame_bounds(
        frames,
        match_.start_frame,
        match_.start_frame.saturating_add(window_frames).min(frames),
    )
}

fn build_profile(
    samples: &[f32],
    channels: usize,
    frames: usize,
    start_frame: usize,
    window_frames: usize,
) -> Option<SimilarityProfile> {
    if channels == 0 || window_frames == 0 || start_frame >= frames {
        return None;
    }
    let radius = sample_radius(window_frames);
    let mut signed = Vec::with_capacity(PROFILE_BINS);
    let mut energy = Vec::with_capacity(PROFILE_BINS);
    for bin in 0..PROFILE_BINS {
        let center_offset =
            ((bin as f64 + 0.5) * window_frames as f64 / PROFILE_BINS as f64).floor() as usize;
        let center = start_frame
            .saturating_add(center_offset)
            .min(frames.saturating_sub(1));
        let frame_start = center.saturating_sub(radius).max(start_frame);
        let frame_end = center
            .saturating_add(radius + 1)
            .min(start_frame.saturating_add(window_frames))
            .min(frames);
        let Some((signed_mean, energy_mean)) =
            sampled_frame_means(samples, channels, frame_start, frame_end)
        else {
            continue;
        };
        signed.push(signed_mean);
        energy.push(energy_mean);
    }
    if signed.len() != PROFILE_BINS || energy.len() != PROFILE_BINS {
        return None;
    }
    let rms = root_mean_square(&signed);
    if rms < MIN_PROFILE_RMS {
        return None;
    }
    normalize_zero_mean(&mut signed)?;
    normalize_zero_mean(&mut energy).unwrap_or(());
    Some(SimilarityProfile {
        signed,
        energy,
        rms,
    })
}

fn sample_radius(window_frames: usize) -> usize {
    (window_frames / PROFILE_BINS / 4).clamp(1, 8)
}

fn sampled_frame_means(
    samples: &[f32],
    channels: usize,
    frame_start: usize,
    frame_end: usize,
) -> Option<(f32, f32)> {
    if frame_end <= frame_start {
        return None;
    }
    let mut signed_sum = 0.0;
    let mut energy_sum = 0.0;
    let mut count = 0usize;
    for frame in frame_start..frame_end {
        let sample = mono_sample_at(samples, channels, frame);
        signed_sum += sample;
        energy_sum += sample.abs();
        count += 1;
    }
    let scale = 1.0 / count as f32;
    Some((signed_sum * scale, energy_sum * scale))
}

fn mono_sample_at(samples: &[f32], channels: usize, frame: usize) -> f32 {
    let base = frame.saturating_mul(channels);
    let mut sum = 0.0;
    for channel in 0..channels {
        sum += samples.get(base + channel).copied().unwrap_or(0.0);
    }
    sum / channels as f32
}

fn normalize_zero_mean(values: &mut [f32]) -> Option<()> {
    let mean = values.iter().copied().sum::<f32>() / values.len().max(1) as f32;
    for value in values.iter_mut() {
        *value -= mean;
    }
    let rms = root_mean_square(values);
    if rms < MIN_PROFILE_RMS {
        return None;
    }
    for value in values {
        *value /= rms;
    }
    Some(())
}

fn root_mean_square(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    (values.iter().map(|value| value * value).sum::<f32>() / values.len() as f32).sqrt()
}

fn profile_similarity(anchor: &SimilarityProfile, candidate: &SimilarityProfile) -> f32 {
    let signed_similarity = normalized_dot(&anchor.signed, &candidate.signed).max(0.0);
    let energy_similarity = normalized_dot(&anchor.energy, &candidate.energy).max(0.0);
    let loudness_ratio = anchor.rms.min(candidate.rms) / anchor.rms.max(candidate.rms).max(1.0e-9);
    (signed_similarity * 0.72 + energy_similarity * 0.23 + loudness_ratio * 0.05).clamp(0.0, 1.0)
}

fn normalized_dot(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    a.iter()
        .zip(b.iter())
        .take(len)
        .map(|(left, right)| left * right)
        .sum::<f32>()
        / len as f32
}

fn ranges_overlap(start_a: usize, end_a: usize, start_b: usize, end_b: usize) -> bool {
    start_a < end_b && start_b < end_a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_marks_repeated_sections_but_not_anchor() {
        let frames = 12_000;
        let shape = test_shape(720);
        let mut samples = vec![0.0; frames];
        write_shape(&mut samples, 1_000, &shape, 1.0);
        write_shape(&mut samples, 5_000, &shape, 0.98);
        write_shape(&mut samples, 8_000, &different_shape(720), 1.0);
        write_shape(&mut samples, 9_500, &shape, 1.02);

        let request = request_for_samples(
            samples,
            SelectionRange::from_frame_bounds(frames, 1_000, 1_720),
        );
        let payload = scan_similar_sections(&request).expect("scan succeeds");

        let starts = payload
            .ranges
            .iter()
            .map(|range| range.frame_bounds(frames).start_frame)
            .collect::<Vec<_>>();
        assert!(
            starts.iter().any(|start| start.abs_diff(5_000) <= 180),
            "second repeated shape should be marked: {starts:?}"
        );
        assert!(
            starts.iter().any(|start| start.abs_diff(9_500) <= 180),
            "third repeated shape should be marked: {starts:?}"
        );
        assert!(
            starts.iter().all(|start| start.abs_diff(1_000) > 180),
            "anchor should not be marked: {starts:?}"
        );
        assert!(
            starts.iter().all(|start| start.abs_diff(8_000) > 180),
            "different section should not be marked: {starts:?}"
        );
    }

    #[test]
    fn scan_rejects_silent_anchor() {
        let frames = 2_000;
        let request = request_for_samples(
            vec![0.0; frames],
            SelectionRange::from_frame_bounds(frames, 100, 900),
        );

        let error = scan_similar_sections(&request).expect_err("silence cannot compare");

        assert_eq!(error, "Selected playmark is too quiet to compare");
    }

    fn request_for_samples(samples: Vec<f32>, anchor: SelectionRange) -> SimilarSectionsRequest {
        let frames = samples.len();
        SimilarSectionsRequest {
            path: PathBuf::from("scan-test.wav"),
            content_revision: 1,
            source: SimilarSectionsSource::InterleavedF32Samples(Arc::from(samples)),
            sample_rate: 48_000,
            channels: 1,
            frames,
            anchor,
        }
    }

    fn test_shape(frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                let t = frame as f32 / frames as f32;
                let envelope = (1.0 - t).powf(1.7);
                (std::f32::consts::TAU * 13.0 * t).sin() * envelope
            })
            .collect()
    }

    fn different_shape(frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                let t = frame as f32 / frames as f32;
                let envelope = (1.0 - t).powf(0.6);
                (std::f32::consts::TAU * 5.0 * t + 0.7).sin() * envelope
            })
            .collect()
    }

    fn write_shape(target: &mut [f32], start: usize, shape: &[f32], gain: f32) {
        for (offset, sample) in shape.iter().enumerate() {
            if let Some(slot) = target.get_mut(start + offset) {
                *slot = *sample * gain;
            }
        }
    }
}
