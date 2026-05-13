use super::*;

#[test]
fn sanitize_samples_removes_nan_and_denormals() {
    let mut out = vec![0.0_f32, f32::NAN, f32::MIN_POSITIVE / 2.0];
    sanitize_samples_in_place(&mut out);
    assert_eq!(out.len(), 3);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(
        out.iter()
            .all(|v| v.abs() == 0.0 || v.abs() >= f32::MIN_POSITIVE)
    );
}

#[test]
fn normalize_peak_scales_to_unit_peak() {
    let mut samples = vec![0.25_f32, -0.5, 0.125];
    normalize_peak_in_place(&mut samples);
    let peak = samples.iter().copied().map(|v| v.abs()).fold(0.0, f32::max);
    assert!((peak - 1.0).abs() < 1e-6);
}

#[test]
fn normalize_rms_targets_expected_level() {
    let mut samples = vec![0.1_f32; 1000];
    let target_db = -20.0;
    normalize_rms_in_place(&mut samples, target_db);
    let measured = rms(&samples);
    let target = db_to_linear(target_db);
    assert!((measured - target).abs() < 1e-3);
}

#[test]
fn normalize_large_parallel_correctness() {
    let count = 1_500_000;
    let mut samples = vec![0.0_f32; count];
    for (i, sample) in samples.iter_mut().enumerate() {
        *sample = (i as f32).sin() * 0.5;
    }

    normalize_peak_in_place(&mut samples);
    let new_peak = samples.iter().copied().map(|v| v.abs()).fold(0.0, f32::max);
    assert!(
        (new_peak - 1.0).abs() < 1e-5,
        "Peak should be 1.0, got {new_peak}"
    );

    let target_db = -15.0;
    normalize_rms_in_place(&mut samples, target_db);
    let measured = rms(&samples);
    let target = db_to_linear(target_db);
    assert!(
        (measured - target).abs() < 1e-4,
        "RMS should be {target}, got {measured}"
    );
}
