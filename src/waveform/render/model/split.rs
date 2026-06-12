pub(in crate::waveform::render) fn split_band_heights(height: u32) -> (u32, u32, u32) {
    let gap = if height >= 3 { 2 } else { 0 };
    let split_height = height.saturating_sub(gap);
    let top_height = (split_height / 2).max(1);
    let bottom_height = split_height.saturating_sub(top_height).max(1);
    (top_height, bottom_height, gap)
}

#[cfg(test)]
mod tests {
    use super::split_band_heights;

    #[test]
    fn split_band_layout_keeps_gap_only_when_height_allows() {
        assert_eq!(split_band_heights(2), (1, 1, 0));
        assert_eq!(split_band_heights(5), (1, 2, 2));
        assert_eq!(split_band_heights(6), (2, 2, 2));
    }
}
