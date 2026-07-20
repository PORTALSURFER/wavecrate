/// Format an absolute timestamp into `HH:MM:SS:MS` where `MS` is zero-padded milliseconds.
pub(crate) fn format_timestamp_hms_ms(seconds: f32) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "00:00:00:000".to_string();
    }
    let total_ms = (seconds * 1_000.0).round() as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms / 60_000) % 60;
    let secs = (total_ms / 1_000) % 60;
    let millis = total_ms % 1_000;
    format!("{hours:02}:{minutes:02}:{secs:02}:{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_timestamp_zero_pads_and_rounds() {
        assert_eq!(format_timestamp_hms_ms(0.0), "00:00:00:000");
        assert_eq!(format_timestamp_hms_ms(1.234), "00:00:01:234");
        assert_eq!(format_timestamp_hms_ms(59.9995), "00:01:00:000");
    }

    #[test]
    fn format_timestamp_handles_hours() {
        assert_eq!(format_timestamp_hms_ms(3_661.789), "01:01:01:789");
        assert_eq!(format_timestamp_hms_ms(-0.5), "00:00:00:000");
    }
}
