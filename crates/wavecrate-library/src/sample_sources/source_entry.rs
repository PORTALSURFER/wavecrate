//! Shared no-follow policy for entries discovered below a sample source root.

use std::{
    fmt, fs, io,
    path::{Component, Path},
};

use super::{is_apple_double_sidecar, is_recognized_audio, is_supported_audio};

/// Version of the source format-classification policy persisted with index-only rows.
pub const SOURCE_FORMAT_POLICY_VERSION: u32 = 1;

/// A filesystem entry type observed without following links or reparse points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEntryFileType {
    /// A directory entry.
    Directory,
    /// A regular file entry.
    File,
    /// A symbolic link or platform reparse point.
    Link,
    /// Any other filesystem object.
    Other,
}

impl SourceEntryFileType {
    /// Convert no-follow file-type predicates into the shared entry type.
    pub fn from_no_followed_type(is_directory: bool, is_file: bool, is_link: bool) -> Self {
        if is_link {
            Self::Link
        } else if is_directory {
            Self::Directory
        } else if is_file {
            Self::File
        } else {
            Self::Other
        }
    }
}

/// A visible regular source-entry kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEntryKind {
    /// A directory that may be traversed without following links.
    Directory,
    /// A regular file.
    File,
}

/// Format support assigned to one regular file by the shared source policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFileClassification {
    /// A file whose format is eligible for the supported sample manifest.
    SupportedAudio,
    /// A recognized audio container that Wavecrate does not currently support.
    UnsupportedAudio,
    /// A regular file that is not recognized as audio.
    UnsupportedNonAudio,
    /// Audio whose format is supported but whose practical constraints reject indexing.
    ///
    /// The current format policy has no such limit. The explicit state lets
    /// format inspection record one without conflating it with inaccessible I/O.
    PracticallyUnsupportedAudio,
}

/// Why a source entry is not visible to source traversals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEntryRejection {
    /// Links and platform reparse points are never traversed as source entries.
    Link,
    /// AppleDouble sidecars are implementation metadata, not source files.
    AppleDouble,
    /// Wavecrate's embedded source database and SQLite sidecars are implementation metadata.
    SourceDatabase,
    /// The entry is neither a regular file nor a directory.
    UnsupportedType,
}

/// Policy result for one entry below a sample source root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEntryClassification {
    /// A directory, including whether its basename is hidden.
    Directory {
        /// Whether this directory's own name starts with `.`.
        hidden: bool,
    },
    /// A regular file, including its source-index eligibility.
    File {
        /// Audio-format support assigned by the shared source policy.
        classification: SourceFileClassification,
        /// Whether the file is below a hidden directory.
        below_hidden_directory: bool,
    },
    /// An entry excluded by the shared source boundary policy.
    Rejected(SourceEntryRejection),
}

impl SourceEntryClassification {
    /// Return the visible browser/tree kind, if this entry is not rejected.
    pub fn visible_kind(self) -> Option<SourceEntryKind> {
        match self {
            Self::Directory { .. } => Some(SourceEntryKind::Directory),
            Self::File { .. } => Some(SourceEntryKind::File),
            Self::Rejected(_) => None,
        }
    }

    /// Return whether a regular file is eligible for the source audio index.
    pub fn indexes_audio(self) -> bool {
        matches!(
            self,
            Self::File {
                classification: SourceFileClassification::SupportedAudio,
                below_hidden_directory: false,
            }
        )
    }

    /// Return whether a regular file has a supported sample-audio format.
    pub fn has_supported_audio(self) -> bool {
        matches!(
            self,
            Self::File {
                classification: SourceFileClassification::SupportedAudio,
                ..
            }
        )
    }

    /// Return the regular-file classification, if this is a visible file.
    pub fn file_classification(self) -> Option<SourceFileClassification> {
        match self {
            Self::File { classification, .. } => Some(classification),
            Self::Directory { .. } | Self::Rejected(_) => None,
        }
    }
}

/// Classify a path relative to a sample-source root from no-follow type facts.
///
/// This policy deliberately preserves the browser's visibility of unsupported
/// files and hidden directories while preventing audio indexing below hidden
/// directories. Callers retain their own traversal mechanics and diagnostics.
pub fn classify_source_entry(
    relative_path: &Path,
    file_type: SourceEntryFileType,
) -> SourceEntryClassification {
    match file_type {
        SourceEntryFileType::Link => {
            SourceEntryClassification::Rejected(SourceEntryRejection::Link)
        }
        SourceEntryFileType::Other => {
            SourceEntryClassification::Rejected(SourceEntryRejection::UnsupportedType)
        }
        SourceEntryFileType::Directory => SourceEntryClassification::Directory {
            hidden: path_name_is_hidden(relative_path),
        },
        SourceEntryFileType::File if is_apple_double_sidecar(relative_path) => {
            SourceEntryClassification::Rejected(SourceEntryRejection::AppleDouble)
        }
        SourceEntryFileType::File if is_source_database_artifact(relative_path) => {
            SourceEntryClassification::Rejected(SourceEntryRejection::SourceDatabase)
        }
        SourceEntryFileType::File => SourceEntryClassification::File {
            classification: if is_supported_audio(relative_path) {
                SourceFileClassification::SupportedAudio
            } else if is_recognized_audio(relative_path) {
                SourceFileClassification::UnsupportedAudio
            } else {
                SourceFileClassification::UnsupportedNonAudio
            },
            below_hidden_directory: has_hidden_ancestor(relative_path),
        },
    }
}

/// Return whether a regular-file path is excluded by name from source indexing.
///
/// This predicate is usable when entry-type inspection fails, before callers
/// can construct a complete [`SourceEntryClassification`].
pub fn is_rejected_source_file_path(relative_path: &Path) -> bool {
    is_apple_double_sidecar(relative_path) || is_source_database_artifact(relative_path)
}

fn is_source_database_artifact(relative_path: &Path) -> bool {
    let Some(name) = relative_path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    [".wavecrate.db", ".wavecrate_samples.db"]
        .iter()
        .any(|database_name| {
            name == *database_name
                || ["-wal", "-shm", "-journal"]
                    .iter()
                    .any(|suffix| name == format!("{database_name}{suffix}"))
        })
}

/// Inspect and classify one path without following links or reparse points.
pub fn classify_path_without_following(
    path: &Path,
) -> Result<SourceEntryClassification, SourceEntryProbeError> {
    let metadata = fs::symlink_metadata(path).map_err(SourceEntryProbeError::from)?;
    let file_type = metadata.file_type();
    Ok(classify_source_entry(
        path,
        SourceEntryFileType::from_no_followed_type(
            file_type.is_dir(),
            file_type.is_file(),
            file_type.is_symlink(),
        ),
    ))
}

/// Bounded failure category for a no-follow source-entry inspection.
#[derive(Debug)]
pub enum SourceEntryProbeError {
    /// The entry disappeared before it could be inspected.
    Missing,
    /// The entry could not be inspected, for example because it is unavailable.
    Unavailable(io::Error),
}

impl From<io::Error> for SourceEntryProbeError {
    fn from(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            Self::Missing
        } else {
            Self::Unavailable(error)
        }
    }
}

impl fmt::Display for SourceEntryProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("entry does not exist"),
            Self::Unavailable(error) => write!(formatter, "entry is unavailable: {error}"),
        }
    }
}

impl std::error::Error for SourceEntryProbeError {}

fn path_name_is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn has_hidden_ancestor(relative_path: &Path) -> bool {
    relative_path.parent().is_some_and(|parent| {
        parent.components().any(|component| {
            let Component::Normal(name) = component else {
                return false;
            };
            name.to_str().is_some_and(|name| name.starts_with('.'))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_agrees_on_visible_and_indexed_entries() {
        assert_eq!(
            classify_source_entry(Path::new("drums"), SourceEntryFileType::Directory),
            SourceEntryClassification::Directory { hidden: false }
        );
        assert_eq!(
            classify_source_entry(Path::new("._kick.wav"), SourceEntryFileType::File),
            SourceEntryClassification::Rejected(SourceEntryRejection::AppleDouble)
        );
        assert!(
            classify_source_entry(Path::new("kick.wav"), SourceEntryFileType::File).indexes_audio()
        );
        assert!(
            !classify_source_entry(Path::new(".cache/kick.wav"), SourceEntryFileType::File)
                .indexes_audio()
        );
        assert_eq!(
            classify_source_entry(Path::new("linked.wav"), SourceEntryFileType::Link),
            SourceEntryClassification::Rejected(SourceEntryRejection::Link)
        );
        assert_eq!(
            classify_source_entry(Path::new("socket"), SourceEntryFileType::Other),
            SourceEntryClassification::Rejected(SourceEntryRejection::UnsupportedType)
        );
        assert_eq!(
            classify_source_entry(Path::new(".wavecrate.db-wal"), SourceEntryFileType::File),
            SourceEntryClassification::Rejected(SourceEntryRejection::SourceDatabase)
        );
    }

    #[test]
    fn unsupported_files_remain_visible_but_are_not_indexed() {
        let entry = classify_source_entry(Path::new("notes.txt"), SourceEntryFileType::File);
        assert_eq!(entry.visible_kind(), Some(SourceEntryKind::File));
        assert!(!entry.has_supported_audio());
        assert!(!entry.indexes_audio());
    }

    #[test]
    fn unsupported_audio_and_non_audio_are_distinct() {
        assert_eq!(
            classify_source_entry(Path::new("loop.flac"), SourceEntryFileType::File)
                .file_classification(),
            Some(SourceFileClassification::UnsupportedAudio)
        );
        assert_eq!(
            classify_source_entry(Path::new("notes.txt"), SourceEntryFileType::File)
                .file_classification(),
            Some(SourceFileClassification::UnsupportedNonAudio)
        );
    }

    #[test]
    fn path_only_rejections_cover_internal_database_and_apple_double_names() {
        for path in [
            ".wavecrate.db",
            ".wavecrate.db-wal",
            ".wavecrate_samples.db-shm",
            "nested/._loop.flac",
        ] {
            assert!(is_rejected_source_file_path(Path::new(path)), "{path}");
        }
        assert!(!is_rejected_source_file_path(Path::new("nested/loop.flac")));
    }
}
