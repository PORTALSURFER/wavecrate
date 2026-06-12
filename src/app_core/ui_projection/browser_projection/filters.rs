use super::*;
use crate::app_core::state::browser_playback_age_filter_chips;

/// Project active browser rating-filter levels into a fixed chip-state array.
pub(super) fn browser_rating_filter_flags(
    rating_filter: &std::collections::BTreeSet<i8>,
) -> [bool; 8] {
    let mut flags = [false; 8];
    for (index, level) in [-3, -2, -1, 0, 1, 2, 3, 4].into_iter().enumerate() {
        flags[index] = rating_filter.contains(&level);
    }
    flags
}

/// Project active browser playback-age filters into a fixed chip-state array.
pub(super) fn browser_playback_age_filter_flags(
    playback_age_filter: &std::collections::BTreeSet<PlaybackAgeFilterChip>,
) -> [bool; 3] {
    let mut flags = [false; 3];
    for (index, chip) in browser_playback_age_filter_chips().into_iter().enumerate() {
        flags[index] = playback_age_filter.contains(&chip);
    }
    flags
}
