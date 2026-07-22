use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::warn;
use wavecrate_library::filesystem_identity::{
    filesystem_change_marker, stable_filesystem_identity,
};

use crate::sample_sources::{SourceDatabase, is_supported_audio};

use super::scan::ScanError;
use super::scan::{SourceTreeFile, SourceTreeSnapshot};

const MAX_LAYOUT_DIAGNOSTICS: usize = 16;

#[derive(Clone, Debug)]
pub(super) struct FileFacts {
    pub(super) relative: PathBuf,
    pub(super) size: u64,
    pub(super) modified_ns: i64,
    pub(super) file_identity: Option<String>,
    change_marker: Option<String>,
}

impl FileFacts {
    pub(super) fn same_file_facts(&self, other: &Self) -> bool {
        self.size == other.size
            && self.modified_ns == other.modified_ns
            && self.file_identity == other.file_identity
    }

    pub(super) fn same_content_snapshot(&self, other: &Self) -> bool {
        self.same_file_facts(other)
            && self
                .change_marker
                .as_ref()
                .zip(other.change_marker.as_ref())
                .is_some_and(|(before, after)| before == after)
    }
}

pub(super) fn ensure_root_dir(db: &SourceDatabase) -> Result<PathBuf, ScanError> {
    let root = db.root().to_path_buf();
    if root.is_dir() {
        Ok(root)
    } else {
        Err(ScanError::InvalidRoot(root))
    }
}

pub(super) fn visit_dir_with_cancel_check(
    root: &Path,
    should_cancel: &mut impl FnMut() -> bool,
    visitor: &mut impl FnMut(&Path) -> Result<(), ScanError>,
) -> Result<SourceTreeSnapshot, ScanError> {
    let mut snapshot = SourceTreeSnapshot {
        directories: vec![PathBuf::new()],
        ..SourceTreeSnapshot::default()
    };
    let mut stack = vec![(root.to_path_buf(), true)];
    while let Some((dir, scan_audio)) = stack.pop() {
        if should_cancel() {
            return Err(ScanError::Canceled);
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if dir != root => {
                warn!(
                    dir = %dir.display(),
                    error = %source,
                    "Failed to read directory during scan"
                );
                record_layout_diagnostic(
                    &mut snapshot,
                    format!("read directory {}: {source}", display_relative(root, &dir)),
                );
                continue;
            }
            Err(source) => {
                return Err(ScanError::Io {
                    path: dir.clone(),
                    source,
                });
            }
        };
        for entry_result in entries {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(err) => {
                    warn!(
                        dir = %dir.display(),
                        error = %err,
                        "Failed to read directory entry during scan"
                    );
                    record_layout_diagnostic(
                        &mut snapshot,
                        format!(
                            "read directory entry {}: {err}",
                            display_relative(root, &dir)
                        ),
                    );
                    continue;
                }
            };

            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    warn!(
                        path = %path.display(),
                        error = %err,
                        "Failed to read file type during scan"
                    );
                    record_layout_diagnostic(
                        &mut snapshot,
                        format!("read file type {}: {err}", display_relative(root, &path)),
                    );
                    continue;
                }
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let relative = path
                    .strip_prefix(root)
                    .map(Path::to_path_buf)
                    .map_err(|_| ScanError::InvalidRoot(path.clone()))?;
                snapshot.directories.push(relative);
                let hidden = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'));
                stack.push((path, scan_audio && !hidden));
                continue;
            }
            if file_type.is_file() {
                if wavecrate_library::sample_sources::is_apple_double_sidecar(&path) {
                    continue;
                }
                if is_supported_audio(&path) {
                    if scan_audio {
                        visitor(&path)?;
                    }
                } else {
                    match entry.metadata() {
                        Ok(metadata) => snapshot.other_files.push(SourceTreeFile {
                            relative_path: path
                                .strip_prefix(root)
                                .map(Path::to_path_buf)
                                .map_err(|_| ScanError::InvalidRoot(path.clone()))?,
                            file_size: metadata.len(),
                        }),
                        Err(err) => record_layout_diagnostic(
                            &mut snapshot,
                            format!(
                                "read file metadata {}: {err}",
                                display_relative(root, &path)
                            ),
                        ),
                    }
                }
            }
        }
    }
    snapshot.directories.sort();
    snapshot
        .other_files
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(snapshot)
}

fn record_layout_diagnostic(snapshot: &mut SourceTreeSnapshot, diagnostic: String) {
    if snapshot.diagnostics.len() < MAX_LAYOUT_DIAGNOSTICS {
        snapshot.diagnostics.push(diagnostic);
    }
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub(super) fn read_facts(root: &Path, path: &Path) -> Result<FileFacts, ScanError> {
    let relative = strip_relative(root, path)?;
    let meta = path.metadata().map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let modified_ns = to_nanos(
        &meta.modified().map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?,
        path,
    )?;
    Ok(FileFacts {
        relative,
        size: meta.len(),
        modified_ns,
        file_identity: stable_filesystem_identity(path, &meta),
        change_marker: filesystem_change_marker(path, &meta),
    })
}

pub(super) fn is_supported_regular_audio_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
        && is_supported_audio(path)
}

pub(super) fn is_supported_scannable_audio_file(root: &Path, relative_path: &Path) -> bool {
    let hidden_ancestor = relative_path.parent().is_some_and(|parent| {
        parent.components().any(|component| {
            let std::path::Component::Normal(name) = component else {
                return false;
            };
            name.to_str().is_some_and(|name| name.starts_with('.'))
        })
    });
    !hidden_ancestor && is_supported_regular_audio_file(&root.join(relative_path))
}

/// Hash the entire file contents for change detection, honoring cancellation when requested.
pub(super) fn compute_content_hash(
    path: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<String, ScanError> {
    let mut file = fs::File::open(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    compute_content_hash_with_reader(path, &mut file, cancel)
}

fn compute_content_hash_with_reader(
    path: &Path,
    reader: &mut impl Read,
    cancel: Option<&AtomicBool>,
) -> Result<String, ScanError> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        let read = reader.read(&mut buffer).map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn strip_relative(root: &Path, path: &Path) -> Result<PathBuf, ScanError> {
    if let Ok(relative) = path.strip_prefix(root) {
        return Ok(PathBuf::from(relative));
    }
    if let (Ok(canon_root), Ok(canon_path)) = (root.canonicalize(), path.canonicalize())
        && let Ok(relative) = canon_path.strip_prefix(&canon_root)
    {
        return Ok(PathBuf::from(relative));
    }
    Err(ScanError::InvalidRoot(path.to_path_buf()))
}

fn to_nanos(time: &SystemTime, path: &Path) -> Result<i64, ScanError> {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ScanError::Time {
            path: path.to_path_buf(),
        })?;
    Ok(duration.as_nanos().min(i64::MAX as u128) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::atomic::AtomicBool;

    struct CancelingReader {
        remaining: usize,
        chunk: usize,
        cancel: std::sync::Arc<AtomicBool>,
        canceled: bool,
    }

    impl CancelingReader {
        fn new(total_bytes: usize, chunk: usize, cancel: std::sync::Arc<AtomicBool>) -> Self {
            Self {
                remaining: total_bytes,
                chunk,
                cancel,
                canceled: false,
            }
        }
    }

    impl io::Read for CancelingReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Ok(0);
            }
            let to_write = self.chunk.min(self.remaining).min(buf.len());
            buf[..to_write].fill(0);
            self.remaining -= to_write;
            if !self.canceled {
                self.cancel.store(true, Ordering::Relaxed);
                self.canceled = true;
            }
            Ok(to_write)
        }
    }

    #[test]
    fn compute_content_hash_cancels_during_read() {
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let mut reader = CancelingReader::new(256 * 1024, 64 * 1024, cancel.clone());
        let result = compute_content_hash_with_reader(
            Path::new("fake.wav"),
            &mut reader,
            Some(cancel.as_ref()),
        );
        assert!(matches!(result, Err(ScanError::Canceled)));
    }

    #[test]
    fn missing_change_markers_never_prove_a_stable_snapshot() {
        let facts = FileFacts {
            relative: PathBuf::from("sample.wav"),
            size: 32,
            modified_ns: 1,
            file_identity: Some(String::from("identity")),
            change_marker: None,
        };

        assert!(!facts.same_content_snapshot(&facts));
    }
}
