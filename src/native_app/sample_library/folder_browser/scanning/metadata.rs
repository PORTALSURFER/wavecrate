use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::super::FileEntry;
use super::{
    entry::{BrowserEntryKind, classify_path_without_following},
    file_entry_metadata::file_entry_with_metadata,
};
use wavecrate::sample_sources::{
    Rating, SampleCollection, SourceDatabase, config::clamp_rating_decay_weeks,
};

const SECONDS_PER_WEEK: i64 = 7 * 24 * 60 * 60;

pub(in crate::native_app::sample_library::folder_browser) type SourceMetadataMap = HashMap<
    PathBuf,
    (
        Rating,
        bool,
        Vec<SampleCollection>,
        Option<i64>,
        Option<i64>,
    ),
>;

pub(super) fn source_rating_map(root: &Path, database_root: &Path) -> SourceMetadataMap {
    let Ok(db) = SourceDatabase::open_for_ui_read_with_database_root(root, database_root) else {
        return HashMap::new();
    };
    let Ok(entries) = db.list_files() else {
        return HashMap::new();
    };
    entries
        .into_iter()
        .map(|entry| {
            let collections = db
                .collections_for_path(&entry.relative_path)
                .unwrap_or_default();
            (
                entry.relative_path,
                (
                    entry.tag,
                    entry.locked,
                    collections,
                    entry.last_played_at,
                    entry.last_curated_at,
                ),
            )
        })
        .collect()
}

pub(super) fn source_rating_map_with_rating_decay(
    root: &Path,
    database_root: &Path,
    rating_decay_weeks: u16,
) -> SourceMetadataMap {
    source_rating_map_with_rating_decay_at(
        root,
        database_root,
        rating_decay_weeks,
        current_epoch_seconds(),
    )
}

fn source_rating_map_with_rating_decay_at(
    root: &Path,
    database_root: &Path,
    rating_decay_weeks: u16,
    now_epoch_seconds: i64,
) -> SourceMetadataMap {
    let Ok(db) =
        SourceDatabase::open_for_user_metadata_write_with_database_root(root, database_root)
    else {
        return source_rating_map(root, database_root);
    };
    let Ok(mut entries) = db.list_files() else {
        return HashMap::new();
    };
    let mut updates = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        if let Some(decayed) = decayed_keep_rating(
            entry.tag,
            entry.locked,
            entry.last_curated_at,
            rating_decay_weeks,
            now_epoch_seconds,
        ) {
            updates.push((index, decayed.rating, decayed.last_curated_at));
        }
    }
    if !updates.is_empty() {
        let result = (|| {
            let mut batch = db.write_batch()?;
            for (index, rating, last_curated_at) in &updates {
                let relative_path = entries[*index].relative_path.as_path();
                batch.set_tag(relative_path, *rating)?;
                batch.set_last_curated_at(relative_path, *last_curated_at)?;
            }
            batch.commit()
        })();
        if result.is_ok() {
            for (index, rating, last_curated_at) in updates {
                entries[index].tag = rating;
                entries[index].last_curated_at = Some(last_curated_at);
            }
        }
    }
    entries
        .into_iter()
        .map(|entry| {
            let collections = db
                .collections_for_path(&entry.relative_path)
                .unwrap_or_default();
            (
                entry.relative_path,
                (
                    entry.tag,
                    entry.locked,
                    collections,
                    entry.last_played_at,
                    entry.last_curated_at,
                ),
            )
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecayedRating {
    rating: Rating,
    last_curated_at: i64,
}

fn decayed_keep_rating(
    rating: Rating,
    locked: bool,
    last_curated_at: Option<i64>,
    rating_decay_weeks: u16,
    now_epoch_seconds: i64,
) -> Option<DecayedRating> {
    if locked || !rating.is_keep() {
        return None;
    }
    let last_curated_at = last_curated_at?;
    let elapsed_seconds = now_epoch_seconds.checked_sub(last_curated_at)?;
    if elapsed_seconds <= 0 {
        return None;
    }
    let period_seconds = i64::from(clamp_rating_decay_weeks(rating_decay_weeks)) * SECONDS_PER_WEEK;
    let elapsed_periods = elapsed_seconds / period_seconds;
    if elapsed_periods <= 0 {
        return None;
    }
    let applied_periods = elapsed_periods.min(i64::from(rating.val()));
    let next_rating = Rating::new(rating.val() - applied_periods as i8);
    if next_rating == rating {
        return None;
    }
    Some(DecayedRating {
        rating: next_rating,
        last_curated_at: last_curated_at.saturating_add(applied_periods * period_seconds),
    })
}

fn current_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().min(i64::MAX as u64) as i64)
        .unwrap_or_default()
}

pub(in crate::native_app::sample_library::folder_browser) fn file_entry_for_source_path(
    path: &PathBuf,
    source_root: &Path,
    source_database_root: &Path,
) -> FileEntry {
    let metadata = source_file_metadata(source_root, source_database_root, path).unwrap_or((
        Rating::NEUTRAL,
        false,
        Vec::new(),
        None,
        None,
    ));
    file_entry_with_metadata(
        path, metadata.0, metadata.1, metadata.2, metadata.3, metadata.4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decays_keep_rating_by_elapsed_intervals() {
        let decayed = decayed_keep_rating(
            Rating::KEEP_3,
            false,
            Some(1_000),
            4,
            1_000 + 9 * SECONDS_PER_WEEK,
        )
        .expect("rating should decay");

        assert_eq!(decayed.rating, Rating::KEEP_1);
        assert_eq!(decayed.last_curated_at, 1_000 + 8 * SECONDS_PER_WEEK);
    }

    #[test]
    fn keeps_locked_keep_rating_unchanged() {
        assert_eq!(
            decayed_keep_rating(
                Rating::KEEP_3,
                true,
                Some(1_000),
                4,
                1_000 + 12 * SECONDS_PER_WEEK
            ),
            None
        );
    }

    #[test]
    fn leaves_neutral_and_trash_ratings_unchanged() {
        assert_eq!(
            decayed_keep_rating(
                Rating::NEUTRAL,
                false,
                Some(1_000),
                4,
                1_000 + 12 * SECONDS_PER_WEEK
            ),
            None
        );
        assert_eq!(
            decayed_keep_rating(
                Rating::TRASH_3,
                false,
                Some(1_000),
                4,
                1_000 + 12 * SECONDS_PER_WEEK
            ),
            None
        );
    }

    #[test]
    fn waits_until_full_interval_elapsed() {
        assert_eq!(
            decayed_keep_rating(
                Rating::KEEP_1,
                false,
                Some(1_000),
                4,
                1_000 + 3 * SECONDS_PER_WEEK
            ),
            None
        );
    }
}

pub(in crate::native_app::sample_library::folder_browser) fn refreshed_file_entries_for_paths(
    paths: &[PathBuf],
    source_root: &Path,
    source_database_root: &Path,
) -> Vec<FileEntry> {
    let ratings = source_rating_map(source_root, source_database_root);
    paths
        .iter()
        .filter(|path| classify_path_without_following(path) == Some(BrowserEntryKind::File))
        .map(|path| rated_file_entry(path, source_root, &ratings))
        .collect()
}

pub(super) fn rated_file_entry(
    path: &PathBuf,
    source_root: &Path,
    ratings: &SourceMetadataMap,
) -> FileEntry {
    let (rating, locked, collections, last_played_at, last_curated_at) = path
        .strip_prefix(source_root)
        .ok()
        .and_then(|relative| ratings.get(relative).cloned())
        .unwrap_or((Rating::NEUTRAL, false, Vec::new(), None, None));
    file_entry_with_metadata(
        path,
        rating,
        locked,
        collections,
        last_played_at,
        last_curated_at,
    )
}

fn source_file_metadata(
    source_root: &Path,
    source_database_root: &Path,
    path: &Path,
) -> Option<(
    Rating,
    bool,
    Vec<SampleCollection>,
    Option<i64>,
    Option<i64>,
)> {
    let relative = path.strip_prefix(source_root).ok()?;
    let db = SourceDatabase::open_for_ui_read_with_database_root(source_root, source_database_root)
        .ok()?;
    let entry = db.entry_for_path(relative).ok()??;
    let collections = db.collections_for_path(relative).unwrap_or_default();
    Some((
        entry.tag,
        entry.locked,
        collections,
        entry.last_played_at,
        entry.last_curated_at,
    ))
}
