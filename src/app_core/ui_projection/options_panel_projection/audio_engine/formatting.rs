pub(super) fn default_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (Default)")
    } else {
        label.to_string()
    }
}

pub(super) fn format_sample_rate_label(sample_rate: u32) -> String {
    if sample_rate >= 1000 && sample_rate.is_multiple_of(1000) {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
    }
}
