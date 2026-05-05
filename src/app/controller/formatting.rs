//! Shared controller formatting helpers.

/// Format a positive finite BPM value for waveform input fields.
///
/// Near-integer values render without decimals to avoid visual jitter while
/// typed/derived fractional values preserve two decimal places.
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
    use super::format_waveform_bpm_input;

    #[test]
    /// Integer-adjacent BPM values should render without fractional digits.
    fn formats_integer_bpm_without_fractional_digits() {
        assert_eq!(format_waveform_bpm_input(120.001), Some("120".to_string()));
    }

    #[test]
    /// Fractional BPM values should render with two decimal places.
    fn formats_fractional_bpm_with_two_decimals() {
        assert_eq!(
            format_waveform_bpm_input(123.45),
            Some("123.45".to_string())
        );
    }

    #[test]
    /// Invalid BPM values should not produce formatted output.
    fn rejects_non_positive_or_non_finite_bpm_values() {
        assert_eq!(format_waveform_bpm_input(f32::NAN), None);
        assert_eq!(format_waveform_bpm_input(f32::INFINITY), None);
        assert_eq!(format_waveform_bpm_input(0.0), None);
        assert_eq!(format_waveform_bpm_input(-1.0), None);
    }
}
