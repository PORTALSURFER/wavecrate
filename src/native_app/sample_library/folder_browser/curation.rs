use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::native_app::audio::playback::{
    tagged_playback_mode_for_tag, tagged_playback_mode_for_tags,
};
use wavecrate::sample_sources::Rating;

use super::FileEntry;
use super::FolderEntry;

const DEFAULT_RECENT_IGNORE_DAYS: u16 = 14;
const SECONDS_PER_DAY: i64 = 86_400;

pub(in crate::native_app) const BROWSER_CURATION_SCOPES: [BrowserCurationScope; 3] = [
    BrowserCurationScope::All,
    BrowserCurationScope::Ratings,
    BrowserCurationScope::Tags,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(in crate::native_app) enum BrowserCurationScope {
    #[default]
    All,
    Ratings,
    Tags,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct BrowserCurationMode {
    pub(in crate::native_app) enabled: bool,
    pub(in crate::native_app) scope: BrowserCurationScope,
    pub(in crate::native_app) recent_ignore_days: u16,
    pub(in crate::native_app) include_k4: bool,
    pub(in crate::native_app) include_recent: bool,
}

impl Default for BrowserCurationMode {
    fn default() -> Self {
        Self {
            enabled: false,
            scope: BrowserCurationScope::All,
            recent_ignore_days: DEFAULT_RECENT_IGNORE_DAYS,
            include_k4: false,
            include_recent: false,
        }
    }
}

impl BrowserCurationScope {
    pub(in crate::native_app) const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Ratings => "Rate",
            Self::Tags => "Tags",
        }
    }

    const fn token(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Ratings => "ratings",
            Self::Tags => "tags",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct CurationEntry {
    priority: CurationPriority,
    curated_rank: i64,
    rating_rank: i8,
    name_rank: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CurationPriority {
    NeverCuratedEmpty = 0,
    NeverCuratedIncomplete = 1,
    NeverCuratedHasMetadata = 2,
    LowRatingIncomplete = 3,
    Stale = 4,
    KeepTwo = 5,
    KeepThree = 6,
    Recent = 7,
    LockedKeep = 8,
}

impl BrowserCurationMode {
    pub(super) fn cache_key(&self) -> String {
        if !self.enabled {
            return String::new();
        }
        format!(
            "curation:{}:{}:{}:{}",
            self.scope.token(),
            self.recent_ignore_days,
            self.include_k4 as u8,
            self.include_recent as u8
        )
    }
}

pub(super) fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

pub(super) fn curation_entry_for_file(
    file: &FileEntry,
    tags: Option<&[String]>,
    mode: &BrowserCurationMode,
    now: i64,
) -> Option<CurationEntry> {
    if !mode.enabled || file.is_missing() {
        return None;
    }
    let locked_keep = is_locked_keep(file);
    if locked_keep && !mode.include_k4 {
        return None;
    }
    let recent = is_recent(file.last_curated_at, mode, now);
    if recent && !mode.include_recent {
        return None;
    }

    let metadata = curation_metadata(tags);
    let priority = curation_priority(file, &metadata, locked_keep, recent);
    if !curation_scope_includes_file(mode.scope, file, &metadata) {
        return None;
    }
    Some(CurationEntry {
        priority,
        curated_rank: file.last_curated_at.unwrap_or(i64::MIN),
        rating_rank: file.rating.val(),
        name_rank: file.name_sort_key(),
    })
}

pub(super) fn curation_badges_for_file(
    file: &FileEntry,
    tags: Option<&[String]>,
    mode: &BrowserCurationMode,
    now: i64,
) -> Vec<String> {
    if !mode.enabled {
        return Vec::new();
    }
    let metadata = curation_metadata(tags);
    let mut badges = Vec::new();
    if file.last_curated_at.is_none() {
        badges.push(String::from("new"));
    }
    if file.rating == Rating::NEUTRAL {
        badges.push(String::from("unrated"));
    } else if file.rating.val() == 1 {
        badges.push(String::from("K1"));
    } else if file.rating.val() == 2 {
        badges.push(String::from("K2"));
    } else if file.rating.val() == 3 {
        badges.push(if file.rating_locked {
            String::from("K4")
        } else {
            String::from("K3")
        });
    }
    if !metadata.has_descriptive_tag {
        badges.push(String::from("untagged"));
    }
    if !metadata.has_playback_type {
        badges.push(String::from("missing type"));
    }
    if is_recent(file.last_curated_at, mode, now) {
        badges.push(String::from("recent"));
    } else if file.last_curated_at.is_some() {
        badges.push(String::from("stale"));
    }
    badges
}

pub(super) fn file_matches_curation(
    file: &FileEntry,
    tags_by_file: &HashMap<String, Vec<String>>,
    mode: &BrowserCurationMode,
    now: i64,
) -> bool {
    let tags = tags_by_file.get(&file.id).map(Vec::as_slice);
    curation_entry_for_file(file, tags, mode, now).is_some()
}

pub(super) fn sort_files_for_curation(
    files: &mut [&FileEntry],
    tags_by_file: &HashMap<String, Vec<String>>,
    mode: &BrowserCurationMode,
    now: i64,
) {
    files.sort_by(|left, right| {
        let left = curation_entry_for_file(
            left,
            tags_by_file.get(&left.id).map(Vec::as_slice),
            mode,
            now,
        );
        let right = curation_entry_for_file(
            right,
            tags_by_file.get(&right.id).map(Vec::as_slice),
            mode,
            now,
        );
        match (left, right) {
            (Some(left), Some(right)) => curation_entry_order(&left, &right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });
}

pub(super) fn sort_file_indices_for_curation(
    folder: &FolderEntry,
    indices: &mut [usize],
    tags_by_file: &HashMap<String, Vec<String>>,
    mode: &BrowserCurationMode,
    now: i64,
) {
    indices.sort_by(|left, right| {
        let left_file = &folder.files[*left];
        let right_file = &folder.files[*right];
        let left = curation_entry_for_file(
            left_file,
            tags_by_file.get(&left_file.id).map(Vec::as_slice),
            mode,
            now,
        );
        let right = curation_entry_for_file(
            right_file,
            tags_by_file.get(&right_file.id).map(Vec::as_slice),
            mode,
            now,
        );
        match (left, right) {
            (Some(left), Some(right)) => curation_entry_order(&left, &right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });
}

pub(super) fn curation_priority_for_file(
    file: &FileEntry,
    tags_by_file: &HashMap<String, Vec<String>>,
    mode: &BrowserCurationMode,
    now: i64,
) -> Option<CurationPriority> {
    curation_entry_for_file(
        file,
        tags_by_file.get(&file.id).map(Vec::as_slice),
        mode,
        now,
    )
    .map(|entry| entry.priority)
}

pub(super) fn curation_entry_order(left: &CurationEntry, right: &CurationEntry) -> Ordering {
    left.priority
        .cmp(&right.priority)
        .then_with(|| left.curated_rank.cmp(&right.curated_rank))
        .then_with(|| left.rating_rank.cmp(&right.rating_rank))
        .then_with(|| left.name_rank.cmp(&right.name_rank))
}

fn curation_priority(
    file: &FileEntry,
    metadata: &CurationMetadata,
    locked_keep: bool,
    recent: bool,
) -> CurationPriority {
    if locked_keep {
        return CurationPriority::LockedKeep;
    }
    if recent {
        return CurationPriority::Recent;
    }
    let complete_metadata = metadata.has_complete_metadata();
    if file.last_curated_at.is_none() {
        if file.rating == Rating::NEUTRAL && !metadata.has_any_metadata {
            CurationPriority::NeverCuratedEmpty
        } else if !complete_metadata {
            CurationPriority::NeverCuratedIncomplete
        } else {
            CurationPriority::NeverCuratedHasMetadata
        }
    } else if file.rating.val() == 2 {
        CurationPriority::KeepTwo
    } else if file.rating.val() == 3 {
        CurationPriority::KeepThree
    } else if !complete_metadata || file.rating.val() <= 1 {
        CurationPriority::LowRatingIncomplete
    } else {
        CurationPriority::Stale
    }
}

fn curation_scope_includes_file(
    scope: BrowserCurationScope,
    file: &FileEntry,
    metadata: &CurationMetadata,
) -> bool {
    match scope {
        BrowserCurationScope::All => true,
        BrowserCurationScope::Ratings => file.rating.val().abs() <= 1,
        BrowserCurationScope::Tags => !metadata.has_complete_metadata(),
    }
}

fn is_locked_keep(file: &FileEntry) -> bool {
    file.rating == Rating::KEEP_3 && file.rating_locked
}

fn is_recent(last_curated_at: Option<i64>, mode: &BrowserCurationMode, now: i64) -> bool {
    let Some(last_curated_at) = last_curated_at else {
        return false;
    };
    let threshold = i64::from(mode.recent_ignore_days) * SECONDS_PER_DAY;
    now.saturating_sub(last_curated_at) < threshold
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CurationMetadata {
    has_any_metadata: bool,
    has_descriptive_tag: bool,
    has_playback_type: bool,
}

impl CurationMetadata {
    fn has_complete_metadata(self) -> bool {
        self.has_descriptive_tag && self.has_playback_type
    }
}

fn curation_metadata(tags: Option<&[String]>) -> CurationMetadata {
    let tags = tags.unwrap_or(&[]);
    let has_playback_type = tagged_playback_mode_for_tags(Some(tags)).is_some();
    let has_descriptive_tag = tags
        .iter()
        .any(|tag| !tag.trim().is_empty() && tagged_playback_mode_for_tag(tag).is_none());
    CurationMetadata {
        has_any_metadata: !tags.is_empty(),
        has_descriptive_tag,
        has_playback_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(id: &str, rating: Rating, locked: bool, last_curated_at: Option<i64>) -> FileEntry {
        FileEntry {
            id: id.to_string(),
            name: id.to_string(),
            stem: id.trim_end_matches(".wav").to_string(),
            extension: String::from("wav"),
            kind: String::from("Audio"),
            size: String::from("1 KB"),
            size_bytes: 1024,
            modified: String::from("Never"),
            modified_rank: u64::MAX,
            rating,
            rating_locked: locked,
            last_curated_at,
            collection: None,
            collections: Vec::new(),
        }
    }

    #[test]
    fn curation_excludes_locked_keep_and_recent_by_default() {
        let mode = BrowserCurationMode::default_enabled_for_tests();
        let now = 1_000_000;

        assert!(
            curation_entry_for_file(
                &file("locked.wav", Rating::KEEP_3, true, Some(0)),
                Some(&[String::from("kick"), String::from("one-shot")]),
                &mode,
                now,
            )
            .is_none()
        );
        assert!(
            curation_entry_for_file(
                &file("recent.wav", Rating::KEEP_1, false, Some(now - 60)),
                Some(&[String::from("kick"), String::from("one-shot")]),
                &mode,
                now,
            )
            .is_none()
        );
    }

    #[test]
    fn curation_prioritizes_never_curated_empty_rows() {
        let mode = BrowserCurationMode::default_enabled_for_tests();
        let now = 1_000_000;
        let empty = curation_entry_for_file(
            &file("a.wav", Rating::NEUTRAL, false, None),
            Some(&[]),
            &mode,
            now,
        )
        .unwrap();
        let tagged = curation_entry_for_file(
            &file("b.wav", Rating::NEUTRAL, false, None),
            Some(&[String::from("kick"), String::from("one-shot")]),
            &mode,
            now,
        )
        .unwrap();

        assert_eq!(empty.priority, CurationPriority::NeverCuratedEmpty);
        assert_eq!(tagged.priority, CurationPriority::NeverCuratedHasMetadata);
        assert_eq!(curation_entry_order(&empty, &tagged), Ordering::Less);
    }

    #[test]
    fn curation_scopes_split_rating_and_tag_work() {
        let now = 1_000_000;
        let complete_tags = [String::from("kick"), String::from("one-shot")];
        let rating_mode = BrowserCurationMode::default_enabled_for_tests_with_scope(
            BrowserCurationScope::Ratings,
        );
        let tag_mode =
            BrowserCurationMode::default_enabled_for_tests_with_scope(BrowserCurationScope::Tags);

        assert!(
            curation_entry_for_file(
                &file("unrated-complete.wav", Rating::NEUTRAL, false, None),
                Some(&complete_tags),
                &rating_mode,
                now,
            )
            .is_some()
        );
        assert!(
            curation_entry_for_file(
                &file("rated-missing-tags.wav", Rating::KEEP_3, false, None),
                Some(&[]),
                &rating_mode,
                now,
            )
            .is_none()
        );
        assert!(
            curation_entry_for_file(
                &file("rated-missing-tags.wav", Rating::KEEP_3, false, None),
                Some(&[]),
                &tag_mode,
                now,
            )
            .is_some()
        );
        assert!(
            curation_entry_for_file(
                &file("unrated-complete.wav", Rating::NEUTRAL, false, None),
                Some(&complete_tags),
                &tag_mode,
                now,
            )
            .is_none()
        );
    }

    impl BrowserCurationMode {
        fn default_enabled_for_tests() -> Self {
            Self {
                enabled: true,
                ..Self::default()
            }
        }

        fn default_enabled_for_tests_with_scope(scope: BrowserCurationScope) -> Self {
            Self {
                scope,
                ..Self::default_enabled_for_tests()
            }
        }
    }
}
