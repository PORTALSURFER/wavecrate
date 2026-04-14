use std::path::Path;

use rusqlite::{Transaction, params};

use super::mutation::execute_transaction_cached;
use crate::sample_sources::db::util::normalize_relative_path;
use crate::sample_sources::{Rating, SourceDbError};

const UPSERT_WAV_FILE_SQL: &str =
    "INSERT INTO wav_files (path, file_size, modified_ns, content_hash, tag, looped, locked, missing, extension)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
     ON CONFLICT(path) DO UPDATE SET file_size = excluded.file_size,
                                    modified_ns = excluded.modified_ns,
                                    content_hash = CASE ?10
                                        WHEN 0 THEN wav_files.content_hash
                                        WHEN 1 THEN NULL
                                        ELSE excluded.content_hash
                                    END,
                                    tag = CASE ?11
                                        WHEN 0 THEN wav_files.tag
                                        ELSE excluded.tag
                                    END,
                                    missing = excluded.missing,
                                    extension = excluded.extension";

struct WavFileUpsertInput {
    path: String,
    file_size: i64,
    modified_ns: i64,
    extension: String,
}

#[derive(Clone, Copy)]
pub(super) enum ContentHashPolicy<'a> {
    Preserve,
    Clear,
    Set(&'a str),
}

impl ContentHashPolicy<'_> {
    fn code(self) -> i64 {
        match self {
            Self::Preserve => 0,
            Self::Clear => 1,
            Self::Set(_) => 2,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum TagPolicy {
    Preserve,
    Set(Rating),
}

impl TagPolicy {
    fn code(self) -> i64 {
        match self {
            Self::Preserve => 0,
            Self::Set(_) => 1,
        }
    }

    fn inserted_tag(self) -> Rating {
        match self {
            Self::Preserve => Rating::NEUTRAL,
            Self::Set(tag) => tag,
        }
    }
}

pub(super) struct WavFileWriteSpec<'a> {
    pub(super) relative_path: &'a Path,
    pub(super) file_size: u64,
    pub(super) modified_ns: i64,
    pub(super) content_hash: ContentHashPolicy<'a>,
    pub(super) tag: TagPolicy,
    pub(super) missing: bool,
}

fn wav_file_upsert_input(
    relative_path: &Path,
    file_size: u64,
    modified_ns: i64,
) -> Result<WavFileUpsertInput, SourceDbError> {
    Ok(WavFileUpsertInput {
        path: normalize_relative_path(relative_path)?,
        file_size: file_size as i64,
        modified_ns,
        extension: relative_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase(),
    })
}

fn missing_flag(missing: bool) -> i64 {
    if missing { 1 } else { 0 }
}

pub(super) fn execute_wav_upsert(
    tx: &Transaction<'_>,
    spec: WavFileWriteSpec<'_>,
) -> Result<(), SourceDbError> {
    let input = wav_file_upsert_input(spec.relative_path, spec.file_size, spec.modified_ns)?;
    let content_hash = match spec.content_hash {
        ContentHashPolicy::Preserve | ContentHashPolicy::Clear => None,
        ContentHashPolicy::Set(value) => Some(value),
    };
    let tag = spec.tag.inserted_tag();
    execute_transaction_cached(
        tx,
        UPSERT_WAV_FILE_SQL,
        params![
            input.path,
            input.file_size,
            input.modified_ns,
            content_hash,
            tag.as_i64(),
            0i64,
            0i64,
            missing_flag(spec.missing),
            input.extension,
            spec.content_hash.code(),
            spec.tag.code()
        ],
    )
}
