use radiant::prelude as ui;
use wavecrate::selection::SelectionRange;

use super::WaveformState;

impl WaveformState {
    #[cfg(test)]
    pub(in crate::native_app) fn marked_play_ranges(&self) -> &[SelectionRange] {
        &self.marked_play_ranges
    }

    pub(in crate::native_app) fn select_marked_play_range_for_random_audition(
        &mut self,
        unit: f32,
    ) -> Option<SelectionRange> {
        let range = random_marked_play_range_for_unit(&self.marked_play_ranges, unit)?;
        self.play_mark_ratio = Some(range.start());
        self.play_selection = Some(range);
        Some(range)
    }
}

pub(in crate::native_app) fn random_marked_play_range_for_unit(
    ranges: &[SelectionRange],
    unit: f32,
) -> Option<SelectionRange> {
    let index = ui::unit_interval_index(unit, ranges.len())?;
    ranges.get(index).copied()
}
