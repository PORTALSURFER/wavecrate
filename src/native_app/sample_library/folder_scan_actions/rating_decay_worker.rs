use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use wavecrate::sample_sources::{Rating, SourceDatabase, config::clamp_rating_decay_weeks};

use crate::native_app::sample_library::folder_browser::scan::RatingDecayMaintenanceRequest;

const SECONDS_PER_WEEK: i64 = 7 * 24 * 60 * 60;

pub(super) fn apply_rating_decay_maintenance(
    request: &RatingDecayMaintenanceRequest,
) -> Result<usize, String> {
    apply_rating_decay_maintenance_at(request, current_epoch_seconds())
}

fn apply_rating_decay_maintenance_at(
    request: &RatingDecayMaintenanceRequest,
    now_epoch_seconds: i64,
) -> Result<usize, String> {
    let db = SourceDatabase::open_for_user_metadata_write_with_database_root(
        &request.root,
        &request.database_root,
    )
    .map_err(|error| format!("open rating-decay metadata writer: {error}"))?;
    let snapshot = db
        .browser_metadata_snapshot()
        .map_err(|error| format!("read rating-decay metadata snapshot: {error}"))?;
    let snapshot_revision = snapshot.revision;
    let updates = snapshot
        .files
        .into_iter()
        .filter_map(|file| {
            decayed_keep_rating(
                file.rating,
                file.locked,
                file.last_curated_at,
                request.rating_decay_weeks,
                now_epoch_seconds,
            )
            .map(|decayed| (file.relative_path, decayed))
        })
        .collect::<Vec<_>>();
    apply_rating_decay_updates(&db, snapshot_revision, &updates)
}

fn apply_rating_decay_updates(
    db: &SourceDatabase,
    snapshot_revision: u64,
    updates: &[(PathBuf, DecayedRating)],
) -> Result<usize, String> {
    if updates.is_empty() {
        return Ok(0);
    }
    let mut batch = db
        .write_batch()
        .map_err(|error| format!("begin rating-decay metadata batch: {error}"))?;
    if !batch
        .matches_revision(snapshot_revision)
        .map_err(|error| format!("fence rating-decay metadata snapshot: {error}"))?
    {
        return Ok(0);
    }
    for (path, decayed) in updates {
        batch
            .set_tag(path, decayed.rating)
            .map_err(|error| format!("stage rating decay for {}: {error}", path.display()))?;
        batch
            .set_last_curated_at(path, decayed.last_curated_at)
            .map_err(|error| {
                format!(
                    "stage rating-decay curation timestamp for {}: {error}",
                    path.display()
                )
            })?;
    }
    batch
        .commit()
        .map_err(|error| format!("commit rating-decay metadata batch: {error}"))?;
    Ok(updates.len())
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
    (next_rating != rating).then_some(DecayedRating {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn rating_decay_maintenance_preserves_decay_semantics() {
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
        assert_eq!(
            decayed_keep_rating(
                Rating::KEEP_3,
                true,
                Some(1_000),
                4,
                1_000 + 12 * SECONDS_PER_WEEK,
            ),
            None
        );
    }

    #[test]
    fn rating_decay_maintenance_persists_one_bounded_batch() {
        let root = tempfile::tempdir().unwrap();
        let relative = Path::new("sample.wav");
        let db = SourceDatabase::open_for_test_fixture_source_write(root.path()).unwrap();
        db.upsert_file(relative, 10, 5).unwrap();
        db.set_tag(relative, Rating::KEEP_3).unwrap();
        db.set_last_curated_at(relative, 1_000).unwrap();
        let request = RatingDecayMaintenanceRequest {
            source_id: String::from("source"),
            root: root.path().to_path_buf(),
            database_root: root.path().to_path_buf(),
            rating_decay_weeks: 4,
        };

        let updated =
            apply_rating_decay_maintenance_at(&request, 1_000 + 9 * SECONDS_PER_WEEK).unwrap();

        assert_eq!(updated, 1);
        let file = db.browser_metadata_snapshot().unwrap().files.pop().unwrap();
        assert_eq!(file.rating, Rating::KEEP_1);
        assert_eq!(file.last_curated_at, Some(1_000 + 8 * SECONDS_PER_WEEK));
    }

    #[test]
    fn rating_decay_maintenance_skips_snapshot_after_newer_user_curation() {
        let root = tempfile::tempdir().unwrap();
        let relative = Path::new("sample.wav");
        let db = SourceDatabase::open_for_test_fixture_source_write(root.path()).unwrap();
        db.upsert_file(relative, 10, 5).unwrap();
        db.set_tag(relative, Rating::KEEP_3).unwrap();
        db.set_last_curated_at(relative, 1_000).unwrap();
        let concurrent_db =
            SourceDatabase::open_for_test_fixture_source_write(root.path()).unwrap();
        let request = RatingDecayMaintenanceRequest {
            source_id: String::from("source"),
            root: root.path().to_path_buf(),
            database_root: root.path().to_path_buf(),
            rating_decay_weeks: 4,
        };

        let snapshot = db.browser_metadata_snapshot().unwrap();
        let snapshot_revision = snapshot.revision;
        let candidate = snapshot.files.into_iter().next().unwrap();
        let updates = vec![(
            candidate.relative_path,
            decayed_keep_rating(
                candidate.rating,
                candidate.locked,
                candidate.last_curated_at,
                request.rating_decay_weeks,
                1_000 + 9 * SECONDS_PER_WEEK,
            )
            .unwrap(),
        )];
        let mut user_batch = concurrent_db.write_batch().unwrap();
        user_batch.set_tag(relative, Rating::TRASH_1).unwrap();
        user_batch.set_locked(relative, true).unwrap();
        user_batch.set_last_curated_at(relative, 9_999).unwrap();
        user_batch.commit().unwrap();

        let updated = apply_rating_decay_updates(&db, snapshot_revision, &updates).unwrap();

        assert_eq!(updated, 0);
        let file = db.browser_metadata_snapshot().unwrap().files.pop().unwrap();
        assert_eq!(file.rating, Rating::TRASH_1);
        assert!(file.locked);
        assert_eq!(file.last_curated_at, Some(9_999));
    }
}
