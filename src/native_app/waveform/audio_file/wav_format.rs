pub(super) fn integer_sample_max_i32(bits_per_sample: u16) -> f32 {
    ((1_i32 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}

pub(super) fn integer_sample_max_i64(bits_per_sample: u16) -> f32 {
    ((1_i64 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}
