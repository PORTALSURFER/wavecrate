use super::super::super::{Rating, SampleSoundType, SourceDatabase};

pub(super) type RowSnapshot = Vec<(
    std::path::PathBuf,
    Rating,
    bool,
    bool,
    bool,
    Option<i64>,
    Option<SampleSoundType>,
)>;

pub(super) fn revision_value(db: &SourceDatabase) -> i64 {
    db.connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'revision'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
        .parse::<i64>()
        .unwrap()
}

pub(super) fn wav_paths_revision_value(db: &SourceDatabase) -> u64 {
    db.get_wav_paths_revision().unwrap()
}

pub(super) fn row_snapshot(db: &SourceDatabase) -> RowSnapshot {
    db.list_files()
        .unwrap()
        .into_iter()
        .map(|row| {
            let relative_path = row.relative_path;
            (
                relative_path.clone(),
                row.tag,
                row.looped,
                row.locked,
                row.missing,
                row.last_played_at,
                db.sound_type_for_path(&relative_path).unwrap(),
            )
        })
        .collect()
}
