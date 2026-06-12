/// Convert normalized `f32` scalar values to millisecond-style thousandths.
pub(in crate::app_core::ui_projection) fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert normalized `f32` scalar values to micro-style millionths.
pub(in crate::app_core::ui_projection) fn normalized_to_micros(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

/// Convert normalized `f64` scalar values to millisecond-style thousandths.
pub(in crate::app_core::ui_projection) fn normalized64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert normalized `f64` scalar values to micro-style millionths.
pub(in crate::app_core::ui_projection) fn normalized64_to_micros(value: f64) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

/// Convert normalized `f64` scalar values to nano-style billionths.
pub(in crate::app_core::ui_projection) fn normalized64_to_nanos(value: f64) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000_000.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_units_round_and_clamp_projection_scalars() {
        assert_eq!(normalized_to_milli(-0.25), 0);
        assert_eq!(normalized_to_milli(0.500_4), 500);
        assert_eq!(normalized_to_milli(1.25), 1000);
        assert_eq!(normalized_to_micros(0.500_4), 500_400);
        assert_eq!(normalized64_to_milli(0.500_6), 501);
        assert_eq!(normalized64_to_micros(0.500_000_6), 500_001);
        assert_eq!(normalized64_to_nanos(0.500_000_000_6), 500_000_001);
    }
}
