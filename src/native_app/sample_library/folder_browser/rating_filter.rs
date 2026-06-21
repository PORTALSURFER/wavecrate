use std::collections::BTreeSet;

use wavecrate::sample_sources::Rating;

use super::FileEntry;

pub(in crate::native_app) const RATING_FILTER_LEVELS: [i8; 8] = [-3, -2, -1, 0, 1, 2, 3, 4];

pub(in crate::native_app) fn rating_filter_label(level: i8) -> &'static str {
    match level {
        -3 => "T3",
        -2 => "T2",
        -1 => "T1",
        0 => "U",
        1 => "K1",
        2 => "K2",
        3 => "K3",
        4 => "K4",
        _ => "",
    }
}

pub(super) fn rating_filter_key(levels: &BTreeSet<i8>) -> String {
    levels
        .iter()
        .copied()
        .filter(|level| RATING_FILTER_LEVELS.contains(level))
        .map(|level| level.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn rating_filter_matches(file: &FileEntry, levels: &BTreeSet<i8>) -> bool {
    levels.is_empty() || levels.contains(&rating_filter_level(file.rating, file.rating_locked))
}

fn rating_filter_level(rating: Rating, locked: bool) -> i8 {
    if locked && rating == Rating::KEEP_3 {
        4
    } else {
        rating.val()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_keep_three_projects_as_fourth_keep_filter_level() {
        assert_eq!(rating_filter_level(Rating::KEEP_3, true), 4);
        assert_eq!(rating_filter_level(Rating::KEEP_3, false), 3);
    }

    #[test]
    fn filter_labels_are_ordered_from_trash_to_keep() {
        assert_eq!(
            RATING_FILTER_LEVELS
                .into_iter()
                .map(rating_filter_label)
                .collect::<Vec<_>>(),
            vec!["T3", "T2", "T1", "U", "K1", "K2", "K3", "K4"]
        );
    }

    #[test]
    fn unrated_filter_level_matches_neutral_ratings() {
        assert_eq!(rating_filter_level(Rating::NEUTRAL, false), 0);
    }
}
