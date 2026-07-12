use std::path::Path;

use super::super::{
    Rating, SampleCollection, SampleSoundType, SourceDatabase, SourceDbError, SourceWriteBatch,
};

/// Content-hash behavior for a scan-owned file upsert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceContentHashWrite<'a> {
    /// Preserve an existing hash when the path already exists.
    Preserve,
    /// Clear the stored hash so deferred hashing can refill it.
    Clear,
    /// Store the supplied full-file content hash.
    Set(&'a str),
}

/// Tag behavior for a scan-owned file upsert.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceTagWrite {
    /// Preserve an existing tag, or use neutral for a new row.
    Preserve,
    /// Store the supplied rating.
    Set(Rating),
}

/// Complete typed input for inserting or refreshing one source file row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFileWrite<'a> {
    /// Path relative to the source root.
    pub relative_path: &'a Path,
    /// Current file size in bytes.
    pub file_size: u64,
    /// Current filesystem modification time in nanoseconds.
    pub modified_ns: i64,
    /// Content-hash persistence policy.
    pub content_hash: SourceContentHashWrite<'a>,
    /// Rating persistence policy.
    pub tag: SourceTagWrite,
    /// Whether the indexed file is unavailable on disk.
    pub missing: bool,
}

/// Mutation applied to the fixed collection memberships of one sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCollectionWrite {
    /// Replace every membership with the optional supplied collection.
    Replace(Option<SampleCollection>),
    /// Add one membership without removing the others.
    Add(SampleCollection),
    /// Remove one membership while retaining the others.
    Remove(SampleCollection),
}

/// Typed source-database mutation shared by one-off and caller-owned transactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceWriteCommand<'a> {
    /// Insert or refresh one indexed source file.
    UpsertFile(SourceFileWrite<'a>),
    /// Store a sample rating.
    SetTag {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Rating to store.
        tag: Rating,
    },
    /// Store a sample loop marker.
    SetLooped {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Loop marker to store.
        looped: bool,
    },
    /// Store a sample keep-lock marker.
    SetLocked {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Keep-lock marker to store.
        locked: bool,
    },
    /// Store or clear a sample sound classification.
    SetSoundType {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Optional classification to store.
        sound_type: Option<SampleSoundType>,
    },
    /// Store or clear a custom user tag.
    SetUserTag {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Optional custom tag to store.
        user_tag: Option<&'a str>,
    },
    /// Store whether the filename was derived from tag metadata.
    SetTagNamed {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Tag-derived filename marker to store.
        tag_named: bool,
    },
    /// Store whether an indexed sample is unavailable on disk.
    SetMissing {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Missing marker to store.
        missing: bool,
    },
    /// Store or clear the last-played timestamp.
    SetLastPlayedAt {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Timestamp to store, or `None` to clear it.
        played_at: Option<i64>,
    },
    /// Store or clear the last-curated timestamp.
    SetLastCuratedAt {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Timestamp to store, or `None` to clear it.
        curated_at: Option<i64>,
    },
    /// Apply one fixed-collection membership mutation.
    SetCollection {
        /// Sample path relative to the source root.
        path: &'a Path,
        /// Collection mutation to apply.
        mutation: SourceCollectionWrite,
    },
    /// Remove one indexed sample row.
    RemoveFile {
        /// Sample path relative to the source root.
        path: &'a Path,
    },
    /// Remap a sample row and path-keyed metadata after a filesystem rename.
    RemapWavFilePath {
        /// Previous path relative to the source root.
        from: &'a Path,
        /// New path relative to the source root.
        to: &'a Path,
    },
    /// Remap path-derived analysis identity after a rename.
    RemapAnalysisSampleIdentity {
        /// Previous path relative to the source root.
        from: &'a Path,
        /// New path relative to the source root.
        to: &'a Path,
    },
    /// Insert or update one source metadata key.
    SetMetadata {
        /// Metadata key.
        key: &'a str,
        /// Metadata value.
        value: &'a str,
    },
}

impl SourceWriteCommand<'_> {
    fn operation(self) -> &'static str {
        match self {
            Self::UpsertFile(_) => "source_db.upsert_file",
            Self::SetTag { .. } => "source_db.set_tag",
            Self::SetLooped { .. } => "source_db.set_looped",
            Self::SetLocked { .. } => "source_db.set_locked",
            Self::SetSoundType { .. } => "source_db.set_sound_type",
            Self::SetUserTag { .. } => "source_db.set_user_tag",
            Self::SetTagNamed { .. } => "source_db.set_tag_named",
            Self::SetMissing { .. } => "source_db.set_missing",
            Self::SetLastPlayedAt {
                played_at: Some(_), ..
            } => "source_db.set_last_played_at",
            Self::SetLastPlayedAt {
                played_at: None, ..
            } => "source_db.clear_last_played_at",
            Self::SetLastCuratedAt {
                curated_at: Some(_),
                ..
            } => "source_db.set_last_curated_at",
            Self::SetLastCuratedAt {
                curated_at: None, ..
            } => "source_db.clear_last_curated_at",
            Self::SetCollection { .. } => "source_db.set_collection",
            Self::RemoveFile { .. } => "source_db.remove_file",
            Self::RemapWavFilePath { .. } => "source_db.remap_wav_file_path",
            Self::RemapAnalysisSampleIdentity { .. } => "source_db.remap_analysis_sample_identity",
            Self::SetMetadata { .. } => "source_db.set_metadata",
        }
    }
}

impl SourceDatabase {
    /// Execute one typed mutation in its own transaction.
    pub fn execute_write(&self, command: SourceWriteCommand<'_>) -> Result<(), SourceDbError> {
        self.mutate_with_batch(command.operation(), |batch| batch.execute_write(command))
    }
}

impl SourceWriteBatch<'_> {
    /// Execute one typed mutation inside this caller-owned transaction.
    pub fn execute_write(&mut self, command: SourceWriteCommand<'_>) -> Result<(), SourceDbError> {
        match command {
            SourceWriteCommand::UpsertFile(write) => self.apply_file_write(write),
            SourceWriteCommand::SetTag { path, tag } => self.set_tag(path, tag),
            SourceWriteCommand::SetLooped { path, looped } => self.set_looped(path, looped),
            SourceWriteCommand::SetLocked { path, locked } => self.set_locked(path, locked),
            SourceWriteCommand::SetSoundType { path, sound_type } => {
                self.set_sound_type(path, sound_type)
            }
            SourceWriteCommand::SetUserTag { path, user_tag } => self.set_user_tag(path, user_tag),
            SourceWriteCommand::SetTagNamed { path, tag_named } => {
                self.set_tag_named(path, tag_named)
            }
            SourceWriteCommand::SetMissing { path, missing } => self.set_missing(path, missing),
            SourceWriteCommand::SetLastPlayedAt { path, played_at } => match played_at {
                Some(value) => self.set_last_played_at(path, value),
                None => self.clear_last_played_at(path),
            },
            SourceWriteCommand::SetLastCuratedAt { path, curated_at } => match curated_at {
                Some(value) => self.set_last_curated_at(path, value),
                None => self.clear_last_curated_at(path),
            },
            SourceWriteCommand::SetCollection { path, mutation } => match mutation {
                SourceCollectionWrite::Replace(value) => self.set_collection(path, value),
                SourceCollectionWrite::Add(value) => self.add_collection(path, value),
                SourceCollectionWrite::Remove(value) => self.remove_collection(path, value),
            },
            SourceWriteCommand::RemoveFile { path } => self.remove_file(path),
            SourceWriteCommand::RemapWavFilePath { from, to } => self.remap_wav_file_path(from, to),
            SourceWriteCommand::RemapAnalysisSampleIdentity { from, to } => {
                self.remap_analysis_sample_identity(from, to)
            }
            SourceWriteCommand::SetMetadata { key, value } => self.set_metadata(key, value),
        }
    }
}
