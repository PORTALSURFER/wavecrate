//! Shared formatting for values rendered by multiple Wavecrate UI runtimes.

/// Format a selection duration with units suited to its scale.
pub(crate) fn format_selection_duration(seconds: f32) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0 ms".to_string();
    }
    if seconds < 1.0 {
        return format!("{:.0} ms", seconds * 1_000.0);
    }
    if seconds < 60.0 {
        return format!("{:.2} s", seconds);
    }
    let minutes = (seconds / 60.0).floor() as u32;
    let remaining = seconds - minutes as f32 * 60.0;
    format!("{minutes}m {remaining:05.2}s")
}

/// Format a positive finite BPM value for display.
///
/// Near-integer values render without decimals to avoid visual jitter while
/// fractional values preserve two decimal places.
pub(crate) fn format_waveform_bpm_input(value: f32) -> Option<String> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let rounded = value.round();
    if (value - rounded).abs() < 0.01 {
        Some(format!("{rounded:.0}"))
    } else {
        Some(format!("{value:.2}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{format_selection_duration, format_waveform_bpm_input};

    #[test]
    fn selection_duration_scales_units() {
        assert_eq!(format_selection_duration(0.75), "750 ms");
        assert_eq!(format_selection_duration(1.5), "1.50 s");
        assert_eq!(format_selection_duration(125.0), "2m 05.00s");
    }

    #[test]
    fn waveform_bpm_formats_integer_and_fractional_values() {
        assert_eq!(format_waveform_bpm_input(120.001), Some("120".to_string()));
        assert_eq!(
            format_waveform_bpm_input(123.45),
            Some("123.45".to_string())
        );
    }

    #[test]
    fn waveform_bpm_rejects_non_positive_or_non_finite_values() {
        assert_eq!(format_waveform_bpm_input(f32::NAN), None);
        assert_eq!(format_waveform_bpm_input(f32::INFINITY), None);
        assert_eq!(format_waveform_bpm_input(0.0), None);
        assert_eq!(format_waveform_bpm_input(-1.0), None);
    }
}
