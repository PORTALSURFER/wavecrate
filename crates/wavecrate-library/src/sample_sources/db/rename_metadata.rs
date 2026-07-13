use std::path::Path;

use rusqlite::params;

use super::util::map_sql_error;
use super::{Rating, SampleCollection, SampleSoundType, SourceDbError, SourceWriteBatch};

/// Complete user metadata retained while scan reconciliation moves a sample identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameMetadataSnapshot {
    /// Rating assigned to the sample.
    pub tag: Rating,
    /// Whether the sample is marked as a loop.
    pub looped: bool,
    /// Canonical sound classification, when assigned.
    pub sound_type: Option<SampleSoundType>,
    /// Whether the highest keep state is locked.
    pub locked: bool,
    /// Most recent playback timestamp, when recorded.
    pub last_played_at: Option<i64>,
    /// Most recent explicit curation timestamp, including an intentionally unset value.
    pub last_curated_at: Option<i64>,
    /// User-authored custom tag, when assigned.
    pub user_tag: Option<String>,
    /// Normal library tag labels assigned to the sample.
    pub normal_tags: Vec<String>,
    /// Every fixed collection membership assigned to the sample.
    pub collections: Vec<SampleCollection>,
    /// Whether the filename was produced from tag metadata.
    pub tag_named: bool,
}

impl SourceWriteBatch<'_> {
    /// Capture the complete rename-recovery metadata contract in the active transaction.
    pub fn snapshot_rename_metadata(
        &mut self,
        relative_path: &Path,
    ) -> Result<RenameMetadataSnapshot, SourceDbError> {
        let path = super::normalize_relative_path(relative_path)?;
        let (tag, looped, sound_type, locked, last_played_at, last_curated_at, user_tag, tag_named) =
            self.tx
                .query_row(
                    "SELECT tag, looped, sound_type, locked, last_played_at,
                        last_curated_at, user_tag, tag_named
                 FROM wav_files
                 WHERE path = ?1",
                    params![path.as_str()],
                    |row| {
                        Ok((
                            Rating::from_i64(row.get::<_, i64>(0)?),
                            row.get::<_, i64>(1)? != 0,
                            row.get::<_, Option<String>>(2)?
                                .as_deref()
                                .and_then(SampleSoundType::from_token),
                            row.get::<_, i64>(3)? != 0,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get::<_, i64>(7)? != 0,
                        ))
                    },
                )
                .map_err(map_sql_error)?;
        let mut statement = self
            .tx
            .prepare_cached(
                "SELECT collection
                 FROM wav_file_collections
                 WHERE path = ?1
                 ORDER BY collection ASC",
            )
            .map_err(map_sql_error)?;
        let stored_collections = statement
            .query_map(params![path], |row| row.get::<_, i64>(0))
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        let collections = stored_collections
            .into_iter()
            .filter_map(SampleCollection::from_i64)
            .collect();
        drop(statement);
        let normal_tags = self.tag_labels_for_path(relative_path)?;

        Ok(RenameMetadataSnapshot {
            tag,
            looped,
            sound_type,
            locked,
            last_played_at,
            last_curated_at,
            user_tag,
            normal_tags,
            collections,
            tag_named,
        })
    }

    /// Restore a complete rename snapshot without recording a new curation event.
    ///
    /// Callers must invoke this inside the same write batch that reconciles the
    /// row. The historical curation timestamp is deliberately written last,
    /// after metadata setters that normally mark explicit user curation.
    pub fn restore_rename_metadata(
        &mut self,
        relative_path: &Path,
        metadata: &RenameMetadataSnapshot,
    ) -> Result<(), SourceDbError> {
        self.set_tag(relative_path, metadata.tag)?;
        self.set_looped(relative_path, metadata.looped)?;
        self.set_sound_type(relative_path, metadata.sound_type)?;
        self.set_locked(relative_path, metadata.locked)?;
        match metadata.last_played_at {
            Some(last_played_at) => self.set_last_played_at(relative_path, last_played_at)?,
            None => self.clear_last_played_at(relative_path)?,
        }
        self.set_user_tag(relative_path, metadata.user_tag.as_deref())?;
        self.set_tag_named(relative_path, metadata.tag_named)?;
        self.replace_tags_for_path(relative_path, &metadata.normal_tags)?;
        self.set_collection(relative_path, None)?;
        for collection in &metadata.collections {
            self.add_collection(relative_path, *collection)?;
        }
        match metadata.last_curated_at {
            Some(last_curated_at) => self.set_last_curated_at(relative_path, last_curated_at),
            None => self.clear_last_curated_at(relative_path),
        }
    }
}
