use super::WaveformState;
#[cfg(test)]
use wavecrate::selection::SelectionRange;

impl WaveformState {
    #[cfg(test)]
    pub(in crate::native_app) fn marked_play_ranges(&self) -> &[SelectionRange] {
        &self.marked_play_ranges
    }
}

#[cfg(test)]
pub(in crate::native_app) fn random_marked_play_range_for_unit(
    ranges: &[SelectionRange],
    unit: f32,
) -> Option<SelectionRange> {
    let index = radiant::prelude::unit_interval_index(unit, ranges.len())?;
    ranges.get(index).copied()
}
