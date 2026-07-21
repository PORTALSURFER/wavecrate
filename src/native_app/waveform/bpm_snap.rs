/// Snap a moving selection edge so the configured beat count spans a whole BPM value.
pub(super) fn snap_resize_ratio_to_whole_bpm(
    fixed_ratio: f32,
    moving_ratio: f32,
    audio_duration_seconds: f32,
    beat_count: u8,
    constrain_to_audio: bool,
) -> f32 {
    let delta = moving_ratio - fixed_ratio;
    let selection_duration_seconds = delta.abs() * audio_duration_seconds;
    if !fixed_ratio.is_finite()
        || !moving_ratio.is_finite()
        || !audio_duration_seconds.is_finite()
        || audio_duration_seconds <= 0.0
        || beat_count == 0
        || !selection_duration_seconds.is_finite()
        || selection_duration_seconds <= 0.0
    {
        return moving_ratio;
    }

    let bpm = f32::from(beat_count) * 60.0 / selection_duration_seconds;
    if !bpm.is_finite() || bpm <= 0.0 {
        return moving_ratio;
    }
    let mut whole_bpm = bpm.round().max(1.0);
    if constrain_to_audio {
        let available_width = if delta.is_sign_positive() {
            1.0 - fixed_ratio
        } else {
            fixed_ratio
        };
        if available_width.is_finite() && available_width > 0.0 {
            let minimum_bpm =
                f32::from(beat_count) * 60.0 / (available_width * audio_duration_seconds);
            if minimum_bpm.is_finite() {
                whole_bpm = whole_bpm.max(minimum_bpm.ceil());
            }
        }
    }
    let snapped_duration_seconds = f32::from(beat_count) * 60.0 / whole_bpm;
    let snapped_width = snapped_duration_seconds / audio_duration_seconds;
    if !snapped_width.is_finite() || snapped_width <= 0.0 {
        return moving_ratio;
    }

    fixed_ratio + delta.signum() * snapped_width
}

#[cfg(test)]
mod tests {
    use super::snap_resize_ratio_to_whole_bpm;

    #[test]
    fn resize_ratio_snaps_beat_span_to_nearest_whole_bpm() {
        let snapped = snap_resize_ratio_to_whole_bpm(0.2, 0.61, 10.0, 4, true);
        let duration = (snapped - 0.2) * 10.0;
        let bpm = 4.0 * 60.0 / duration;

        assert!((bpm - 59.0).abs() < 0.001);
    }

    #[test]
    fn resize_ratio_preserves_drag_direction_around_fixed_edge() {
        let snapped = snap_resize_ratio_to_whole_bpm(0.8, 0.39, 10.0, 4, true);

        assert!(snapped < 0.8);
        assert!((4.0 * 60.0 / ((0.8 - snapped) * 10.0) - 59.0).abs() < 0.001);
    }

    #[test]
    fn invalid_inputs_leave_resize_ratio_unchanged() {
        assert_eq!(
            snap_resize_ratio_to_whole_bpm(0.2, 0.61, 0.0, 4, true),
            0.61
        );
        assert_eq!(
            snap_resize_ratio_to_whole_bpm(0.2, 0.61, 10.0, 0, true),
            0.61
        );
    }

    #[test]
    fn constrained_snap_chooses_a_whole_bpm_that_fits_at_audio_edge() {
        let snapped = snap_resize_ratio_to_whole_bpm(0.2, 0.99, 10.0, 4, true);
        let bpm = 4.0 * 60.0 / ((snapped - 0.2) * 10.0);

        assert!(snapped <= 1.0);
        assert!((bpm - bpm.round()).abs() < 0.001);
    }
}
