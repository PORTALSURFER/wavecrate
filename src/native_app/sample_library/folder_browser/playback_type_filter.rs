use std::collections::{BTreeSet, HashMap};

use crate::native_app::audio::playback::{TaggedPlaybackMode, tagged_playback_mode_for_tags};

use super::FileEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::native_app) enum PlaybackTypeFilter {
    OneShot,
    Loop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PlaybackTypeSortRank {
    Loop,
    OneShot,
    Unknown,
}

pub(in crate::native_app) const PLAYBACK_TYPE_FILTERS: [PlaybackTypeFilter; 2] =
    [PlaybackTypeFilter::OneShot, PlaybackTypeFilter::Loop];

pub(in crate::native_app) fn playback_type_filter_label(
    filter: PlaybackTypeFilter,
) -> &'static str {
    match filter {
        PlaybackTypeFilter::OneShot => "1-Shot",
        PlaybackTypeFilter::Loop => "Loop",
    }
}

pub(super) fn playback_type_filter_matches(
    file: &FileEntry,
    tags_by_file: &HashMap<String, Vec<String>>,
    filters: &BTreeSet<PlaybackTypeFilter>,
) -> bool {
    playback_type_filter_matches_tags(tags_by_file.get(&file.id).map(Vec::as_slice), filters)
}

pub(super) fn playback_type_filter_matches_tags(
    tags: Option<&[String]>,
    filters: &BTreeSet<PlaybackTypeFilter>,
) -> bool {
    if filters.is_empty() {
        return true;
    }
    tagged_playback_mode_for_tags(tags)
        .map(PlaybackTypeFilter::from)
        .is_some_and(|filter| filters.contains(&filter))
}

pub(super) fn playback_type_sort_rank(
    file: &FileEntry,
    tags_by_file: Option<&HashMap<String, Vec<String>>>,
) -> PlaybackTypeSortRank {
    let tags = tags_by_file.and_then(|tags_by_file| tags_by_file.get(&file.id).map(Vec::as_slice));
    playback_type_sort_rank_for_tags(tags)
}

fn playback_type_sort_rank_for_tags(tags: Option<&[String]>) -> PlaybackTypeSortRank {
    match tagged_playback_mode_for_tags(tags) {
        Some(TaggedPlaybackMode::Loop) => PlaybackTypeSortRank::Loop,
        Some(TaggedPlaybackMode::OneShot) => PlaybackTypeSortRank::OneShot,
        None => PlaybackTypeSortRank::Unknown,
    }
}

impl From<TaggedPlaybackMode> for PlaybackTypeFilter {
    fn from(mode: TaggedPlaybackMode) -> Self {
        match mode {
            TaggedPlaybackMode::OneShot => Self::OneShot,
            TaggedPlaybackMode::Loop => Self::Loop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_type_filter_matches_requested_tagged_mode() {
        let loop_only = BTreeSet::from([PlaybackTypeFilter::Loop]);
        let one_shot_only = BTreeSet::from([PlaybackTypeFilter::OneShot]);

        assert!(playback_type_filter_matches_tags(
            Some(&[String::from("loop")]),
            &loop_only
        ));
        assert!(!playback_type_filter_matches_tags(
            Some(&[String::from("one-shot")]),
            &loop_only
        ));
        assert!(playback_type_filter_matches_tags(
            Some(&[String::from("one-shot")]),
            &one_shot_only
        ));
        assert!(!playback_type_filter_matches_tags(None, &one_shot_only));
    }

    #[test]
    fn empty_playback_type_filter_matches_everything() {
        assert!(playback_type_filter_matches_tags(None, &BTreeSet::new()));
    }

    #[test]
    fn playback_type_sort_rank_groups_known_modes_before_unknown() {
        assert!(
            playback_type_sort_rank_for_tags(Some(&[String::from("loop")]))
                < playback_type_sort_rank_for_tags(Some(&[String::from("one-shot")]))
        );
        assert!(
            playback_type_sort_rank_for_tags(Some(&[String::from("one-shot")]))
                < playback_type_sort_rank_for_tags(None)
        );
    }
}
