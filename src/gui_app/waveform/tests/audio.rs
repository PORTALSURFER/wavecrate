use super::*;

#[test]
fn waveform_summary_preserves_raw_transient_detail() {
    let samples = vec![0.0, 0.12, -0.9, 0.08, 0.0, 0.42, -0.18, 0.0];

    let file = waveform_file_from_mono_samples(
        "test.wav".into(),
        Arc::from([]),
        48_000,
        1,
        samples.clone(),
    );

    assert_eq!(BAND_COUNT, 4);
    let raw_peak_index = samples
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
        .map(|(index, _)| index)
        .expect("peak sample");
    let rendered_peak_index = file.gpu_signal_summary.levels[0]
        .buckets
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            left.max
                .abs()
                .max(left.min.abs())
                .total_cmp(&right.max.abs().max(right.min.abs()))
        })
        .map(|(index, _)| index / BAND_COUNT)
        .expect("peak band sample");

    assert_eq!(rendered_peak_index, raw_peak_index);
    let frame_peak = file.gpu_signal_summary.levels[0].buckets
        [raw_peak_index * BAND_COUNT..(raw_peak_index + 1) * BAND_COUNT]
        .iter()
        .map(|bucket| bucket.min.abs().max(bucket.max.abs()))
        .fold(0.0_f32, f32::max);
    assert!(frame_peak > 0.89);
}

#[test]
fn stereo_downmix_preserves_per_frame_peak_height_for_normalized_files() {
    let samples = vec![1.0, 0.0, -0.25, 0.25, 0.0, -1.0, 0.5, -0.75];

    assert_eq!(
        super::downmix_to_mono(&samples, 2, 4),
        vec![1.0, -0.25, -1.0, -0.75]
    );
}

#[test]
fn stereo_downmix_avoids_phase_cancellation_in_visual_projection() {
    let samples = vec![1.0, -1.0, 0.35, -0.2];

    assert_eq!(super::downmix_to_mono(&samples, 2, 2), vec![1.0, 0.35]);
}

#[test]
fn frequency_bands_keep_low_mid_high_and_raw_lanes_separate() {
    let samples = [0.0, 0.7, -0.7, 0.18, -0.18, 0.02, -0.02, 0.0];
    let bands = split_frequency_bands(&samples, 48_000);

    assert_eq!(bands.len(), samples.len() * BAND_COUNT);
    let low_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[0].abs())
        .fold(0.0_f32, f32::max);
    let mid_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[1].abs())
        .fold(0.0_f32, f32::max);
    let high_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[2].abs())
        .fold(0.0_f32, f32::max);
    let raw_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[3].abs())
        .fold(0.0_f32, f32::max);

    assert!(low_peak > 0.0);
    assert!(mid_peak > 0.0);
    assert!(high_peak > 0.0);
    assert!(raw_peak > 0.69);
}

#[test]
fn frequency_bands_raw_lane_preserves_visual_peak_for_normalized_content() {
    let sample_rate = 48_000;
    let low = (0..sample_rate / 100)
        .map(|frame| {
            let t = frame as f32 / sample_rate as f32;
            (std::f32::consts::TAU * 70.0 * t).sin()
        })
        .collect::<Vec<_>>();
    let high = (0..sample_rate / 100)
        .map(|frame| {
            let t = frame as f32 / sample_rate as f32;
            (std::f32::consts::TAU * 4_000.0 * t).sin()
        })
        .collect::<Vec<_>>();

    for samples in [low, high] {
        let bands = split_frequency_bands(&samples, sample_rate);
        let raw_peak = bands
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[3].abs())
            .fold(0.0_f32, f32::max);

        assert!(
            (raw_peak - 1.0).abs() < 0.000_01,
            "raw display peak should track normalized sample peak, got {raw_peak}"
        );
    }
}

#[test]
fn frequency_bands_normalize_short_low_content_to_raw_visual_peak() {
    let sample_rate = 48_000;
    let samples = (0..2_656)
        .map(|frame| {
            let t = frame as f32 / sample_rate as f32;
            (std::f32::consts::TAU * 72.0 * t).sin()
        })
        .collect::<Vec<_>>();

    let bands = split_frequency_bands(&samples, sample_rate);
    let low_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[0].abs())
        .fold(0.0_f32, f32::max);
    let raw_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[3].abs())
        .fold(0.0_f32, f32::max);

    assert!(raw_peak > 0.99, "raw peak was {raw_peak}");
    assert!(
        low_peak > raw_peak * 0.94,
        "short low content should not render visually undersized: low={low_peak}, raw={raw_peak}"
    );
}

#[test]
fn frequency_bands_use_envelopes_to_avoid_low_zero_crossing_gaps() {
    let sample_rate = 48_000;
    let samples = (0..sample_rate / 20)
        .map(|frame| {
            let t = frame as f32 / sample_rate as f32;
            (std::f32::consts::TAU * 60.0 * t).sin()
        })
        .collect::<Vec<_>>();

    let bands = split_frequency_bands(&samples, sample_rate);
    let low_values = bands
        .chunks_exact(BAND_COUNT)
        .skip(sample_rate as usize / 50)
        .map(|frame| frame[0].abs())
        .collect::<Vec<_>>();
    let low_peak = low_values.iter().copied().fold(0.0_f32, f32::max);
    let low_floor = low_values.iter().copied().fold(f32::INFINITY, f32::min);

    assert!(low_peak > 0.94, "low envelope peak was {low_peak}");
    assert!(
        low_floor > low_peak * 0.55,
        "sustained low envelope should not collapse at zero crossings: floor={low_floor}, peak={low_peak}"
    );
}

#[test]
fn frequency_bands_do_not_inflate_low_color_for_high_frequency_content() {
    let sample_rate = 48_000;
    let samples = (0..sample_rate / 100)
        .map(|frame| {
            let t = frame as f32 / sample_rate as f32;
            (std::f32::consts::TAU * 7_200.0 * t).sin()
        })
        .collect::<Vec<_>>();

    let bands = split_frequency_bands(&samples, sample_rate);
    let low_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[0].abs())
        .fold(0.0_f32, f32::max);
    let high_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[2].abs())
        .fold(0.0_f32, f32::max);

    assert!(high_peak > 0.30, "high peak was {high_peak}");
    assert!(
        low_peak < high_peak * 0.35,
        "mostly high-frequency content should not be painted as low-end blue: low={low_peak}, high={high_peak}"
    );
}
