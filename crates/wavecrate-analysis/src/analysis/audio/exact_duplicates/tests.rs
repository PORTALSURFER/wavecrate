use super::*;

fn mono_samples(windows: &[[f32; 4]]) -> Vec<f32> {
    windows
        .iter()
        .flat_map(|window| window.iter().copied())
        .collect()
}

#[test]
fn keeps_first_duplicate_window_and_marks_later_matches() {
    let samples = mono_samples(&[
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.6, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.4, 0.0, 0.0],
        [0.0, 0.6, 0.0, 0.0],
    ]);

    let detection =
        detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[1, 5, 9, 13, 17]).unwrap();

    assert_eq!(detection.duplicate_group_count, 2);
    assert_eq!(detection.duplicate_window_count, 2);
    assert_eq!(
        detection
            .duplicate_windows
            .iter()
            .map(|window| (window.start_frame, window.end_frame, window.group_id))
            .collect::<Vec<_>>(),
        vec![(8, 12, 0), (16, 20, 1)]
    );
}

#[test]
fn aligns_candidates_from_selection_event_offset() {
    let samples = vec![
        9.0, 0.0, 1.0, 0.0, 0.0, 0.0, 7.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0,
    ];

    let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 1, &[2, 8]).unwrap();

    assert_eq!(detection.duplicate_window_count, 1);
    assert_eq!(
        detection
            .duplicate_windows
            .iter()
            .map(|window| (window.start_frame, window.end_frame))
            .collect::<Vec<_>>(),
        vec![(7, 11)]
    );
}

#[test]
fn groups_multiple_duplicate_families_across_whole_scan() {
    let samples = mono_samples(&[
        [0.0, 0.8, 0.0, 0.0],
        [0.0, 0.6, 0.0, 0.0],
        [0.0, 0.8, 0.0, 0.0],
        [0.0, 0.4, 0.0, 0.0],
        [0.0, 0.6, 0.0, 0.0],
        [0.0, 0.3, 0.0, 0.0],
        [0.0, 0.2, 0.0, 0.0],
        [0.0, 0.3, 0.0, 0.0],
        [0.0, 0.4, 0.0, 0.0],
        [0.0, 0.2, 0.0, 0.0],
    ]);

    let detection = detect_exact_duplicate_window_ranges(
        &samples,
        1,
        4,
        4,
        0,
        &[1, 5, 9, 13, 17, 21, 25, 29, 33, 37],
    )
    .unwrap();

    assert_eq!(detection.duplicate_group_count, 5);
    assert_eq!(detection.duplicate_window_count, 5);
}

#[test]
fn accepts_tiny_inaudible_shape_drift() {
    let samples = mono_samples(&[[0.0, 1.0, 0.0, 0.0], [0.0, 0.998, 0.001, 0.0]]);

    let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[1, 5]).unwrap();

    assert_eq!(detection.duplicate_group_count, 1);
    assert_eq!(detection.duplicate_window_count, 1);
    assert_eq!(detection.duplicate_windows[0].start_frame, 4);
}

#[test]
fn rejects_audibly_different_hits() {
    let samples = mono_samples(&[[0.0, 1.0, 0.0, 0.0], [0.0, 0.6, 0.4, 0.0]]);

    let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[1, 5]).unwrap();

    assert_eq!(detection, ExactDuplicateWindowDetection::default());
}

#[test]
fn rejects_silent_selection_windows() {
    let samples = mono_samples(&[[0.0, 0.0, 0.0, 0.0], [0.0, 0.8, 0.0, 0.0]]);

    let err = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[5])
        .expect_err("silent selections should be rejected");

    assert_eq!(
        err,
        "The duplicate scan selection must include audible audio"
    );
}
