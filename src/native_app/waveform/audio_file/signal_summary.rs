use radiant::runtime::{GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel};
use std::sync::Arc;

use super::super::BAND_COUNT;

pub(in crate::native_app::waveform) fn gpu_signal_summary_with_progress_and_cancel(
    samples: &[f32],
    frames: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<GpuSignalSummary, String> {
    let band_count = BAND_COUNT.max(1);
    let frames = frames.min(samples.len() / band_count);
    let mut levels = Vec::with_capacity(signal_summary_level_count(frames));
    let mut bucket_frames = 1usize;
    let mut previous_buckets: Option<Arc<[GpuSignalSummaryBucket]>> = None;
    let total_levels = signal_summary_level_count(frames).max(1);
    while bucket_frames <= frames.max(1) {
        let level_index = levels.len();
        let level_start = start + (end - start) * (level_index as f32 / total_levels as f32);
        let level_end = start + (end - start) * ((level_index + 1) as f32 / total_levels as f32);
        let buckets = build_signal_summary_level(
            SignalSummaryLevelInput {
                samples,
                previous: previous_buckets.as_deref(),
                frames,
                band_count,
                bucket_frames,
            },
            level_start,
            level_end,
            progress,
            cancelled,
        );
        let buckets = buckets?;
        levels.push(GpuSignalSummaryLevel {
            bucket_frames,
            buckets: Arc::clone(&buckets),
        });
        previous_buckets = Some(buckets);
        if bucket_frames >= frames.max(1) {
            break;
        }
        bucket_frames = bucket_frames.saturating_mul(2).max(bucket_frames + 1);
    }
    progress(end);
    Ok(GpuSignalSummary {
        frames,
        band_count,
        levels,
    })
}

struct SignalSummaryLevelInput<'a> {
    samples: &'a [f32],
    previous: Option<&'a [GpuSignalSummaryBucket]>,
    frames: usize,
    band_count: usize,
    bucket_frames: usize,
}

fn build_signal_summary_level(
    input: SignalSummaryLevelInput<'_>,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Arc<[GpuSignalSummaryBucket]>, String> {
    match input.previous {
        Some(previous) => merge_signal_summary_level_with_progress(MergeSignalSummaryLevelInput {
            previous,
            start,
            end,
            frames: input.frames,
            band_count: input.band_count,
            bucket_frames: input.bucket_frames,
            progress,
            cancelled,
        }),
        None => build_signal_summary_base_level_with_progress(
            input.samples,
            input.frames,
            input.band_count,
            start,
            end,
            progress,
            cancelled,
        ),
    }
}

fn signal_summary_level_count(frames: usize) -> usize {
    let frames = frames.max(1);
    usize::BITS as usize - frames.leading_zeros() as usize
}

fn build_signal_summary_base_level_with_progress(
    samples: &[f32],
    frames: usize,
    band_count: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Arc<[GpuSignalSummaryBucket]>, String> {
    if frames == 0 {
        return Ok(vec![GpuSignalSummaryBucket::default(); band_count].into());
    }
    let sample_count = frames.saturating_mul(band_count);
    let mut buckets = Vec::with_capacity(sample_count);
    for (index, value) in samples.iter().copied().take(sample_count).enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        buckets.push(signal_summary_bucket(value));
        super::report_phase_progress_throttled(start, end, index + 1, sample_count, progress);
    }
    progress(end);
    Ok(buckets.into())
}

fn signal_summary_bucket(value: f32) -> GpuSignalSummaryBucket {
    if value.is_finite() {
        GpuSignalSummaryBucket {
            min: value,
            max: value,
        }
    } else {
        GpuSignalSummaryBucket::default()
    }
}

struct MergeSignalSummaryLevelInput<'a, Progress, Cancelled>
where
    Progress: Fn(f32),
    Cancelled: Fn() -> bool,
{
    previous: &'a [GpuSignalSummaryBucket],
    frames: usize,
    band_count: usize,
    bucket_frames: usize,
    start: f32,
    end: f32,
    progress: &'a Progress,
    cancelled: &'a Cancelled,
}

fn merge_signal_summary_level_with_progress<Progress, Cancelled>(
    input: MergeSignalSummaryLevelInput<'_, Progress, Cancelled>,
) -> Result<Arc<[GpuSignalSummaryBucket]>, String>
where
    Progress: Fn(f32),
    Cancelled: Fn() -> bool,
{
    let bucket_count = input.frames.div_ceil(input.bucket_frames.max(1)).max(1);
    let previous_bucket_count = input.previous.len() / input.band_count.max(1);
    let mut buckets = Vec::with_capacity(bucket_count.saturating_mul(input.band_count));
    for bucket in 0..bucket_count {
        if (input.cancelled)() {
            return Err(String::from("cancelled"));
        }
        push_merged_bucket_bands(
            input.previous,
            previous_bucket_count,
            input.band_count,
            bucket,
            &mut buckets,
        );
        super::report_phase_progress_throttled(
            input.start,
            input.end,
            bucket + 1,
            bucket_count,
            input.progress,
        );
    }
    (input.progress)(input.end);
    Ok(buckets.into())
}

fn push_merged_bucket_bands(
    previous: &[GpuSignalSummaryBucket],
    previous_bucket_count: usize,
    band_count: usize,
    bucket: usize,
    buckets: &mut Vec<GpuSignalSummaryBucket>,
) {
    let first = bucket.saturating_mul(2);
    let second = first + 1;
    for band in 0..band_count {
        let mut summary = previous
            .get(first.saturating_mul(band_count).saturating_add(band))
            .copied()
            .unwrap_or_default();
        if second < previous_bucket_count
            && let Some(next) = previous.get(second.saturating_mul(band_count).saturating_add(band))
        {
            summary.min = summary.min.min(next.min);
            summary.max = summary.max.max(next.max);
        }
        buckets.push(summary);
    }
}
