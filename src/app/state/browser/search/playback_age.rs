use std::collections::BTreeSet;

/// Browser playback-age filter chips shown in the native toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaybackAgeFilterChip {
    /// Samples that have never been played.
    NeverPlayed,
    /// Samples whose last playback was at least 30 days ago.
    OlderThanMonth,
    /// Samples whose last playback was at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
}

/// Visual playback-age buckets derived from each sample's `last_played_at` timestamp.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaybackAgeBucket {
    /// Samples played within the last 7 days, including future-skewed timestamps.
    #[default]
    Fresh,
    /// Samples last played at least 7 days ago but less than 30 days ago.
    OlderThanWeek,
    /// Samples last played at least 30 days ago.
    OlderThanMonth,
    /// Samples with no recorded playback timestamp.
    NeverPlayed,
}

impl PlaybackAgeBucket {
    pub(super) const WEEK_SECS: i64 = 7 * 24 * 60 * 60;
    pub(super) const MONTH_SECS: i64 = 30 * 24 * 60 * 60;

    /// Classify one optional playback timestamp into the browser aging buckets.
    pub fn from_last_played_at(last_played_at: Option<i64>, now_unix_secs: i64) -> Self {
        let Some(last_played_at) = last_played_at else {
            return Self::NeverPlayed;
        };
        if last_played_at >= now_unix_secs {
            return Self::Fresh;
        }
        let age_secs = now_unix_secs.saturating_sub(last_played_at);
        if age_secs >= Self::MONTH_SECS {
            Self::OlderThanMonth
        } else if age_secs >= Self::WEEK_SECS {
            Self::OlderThanWeek
        } else {
            Self::Fresh
        }
    }

    /// Return whether this visual bucket should match one toolbar filter chip.
    pub fn matches_filter_chip(self, chip: PlaybackAgeFilterChip) -> bool {
        match chip {
            PlaybackAgeFilterChip::NeverPlayed => self == Self::NeverPlayed,
            PlaybackAgeFilterChip::OlderThanMonth => self == Self::OlderThanMonth,
            PlaybackAgeFilterChip::OlderThanWeek => self == Self::OlderThanWeek,
        }
    }
}

/// Return the fixed browser playback-age chip order used across UI surfaces.
pub fn browser_playback_age_filter_chips() -> [PlaybackAgeFilterChip; 3] {
    [
        PlaybackAgeFilterChip::NeverPlayed,
        PlaybackAgeFilterChip::OlderThanMonth,
        PlaybackAgeFilterChip::OlderThanWeek,
    ]
}

/// Return whether one playback-age bucket passes the active toolbar chip set.
pub fn playback_age_bucket_matches_filters(
    filters: &BTreeSet<PlaybackAgeFilterChip>,
    bucket: PlaybackAgeBucket,
) -> bool {
    filters.is_empty() || filters.iter().any(|chip| bucket.matches_filter_chip(*chip))
}

/// Return the next Unix-second boundary where one row's playback-age filter
/// match result can change for the active chip set.
pub(crate) fn next_playback_age_filter_change_unix_secs(
    filters: &BTreeSet<PlaybackAgeFilterChip>,
    last_played_at: Option<i64>,
    now_unix_secs: i64,
) -> Option<i64> {
    if filters.is_empty() {
        return None;
    }
    let last_played_at = last_played_at?;

    let current_matches = playback_age_bucket_matches_filters(
        filters,
        PlaybackAgeBucket::from_last_played_at(Some(last_played_at), now_unix_secs),
    );
    for transition_unix_secs in [
        last_played_at.saturating_add(PlaybackAgeBucket::WEEK_SECS),
        last_played_at.saturating_add(PlaybackAgeBucket::MONTH_SECS),
    ] {
        if transition_unix_secs <= now_unix_secs {
            continue;
        }
        let future_matches = playback_age_bucket_matches_filters(
            filters,
            PlaybackAgeBucket::from_last_played_at(Some(last_played_at), transition_unix_secs),
        );
        if future_matches != current_matches {
            return Some(transition_unix_secs);
        }
    }
    None
}
