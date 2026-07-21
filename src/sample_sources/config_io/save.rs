use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicU64, Ordering},
};

use super::super::config_types::{AppConfig, AppSettings, ConfigError};
use super::load::config_path;

#[derive(Default)]
struct SaveRevisionGate {
    targets: Mutex<HashMap<PathBuf, Arc<TargetSaveRevisionGate>>>,
}

#[derive(Default)]
struct TargetSaveRevisionGate {
    revision: AtomicU64,
    lock: Mutex<()>,
}

/// Reserved generation for one concrete configuration target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigSaveRevision {
    target: PathBuf,
    revision: u64,
}

impl TargetSaveRevisionGate {
    fn reserve(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::AcqRel).wrapping_add(1)
    }

    fn run_if_current<E>(
        &self,
        revision: u64,
        write: impl FnOnce() -> Result<(), E>,
    ) -> Result<bool, E> {
        let _guard = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.revision.load(Ordering::Acquire) != revision {
            return Ok(false);
        }
        write()?;
        Ok(true)
    }
}

impl SaveRevisionGate {
    fn target(&self, path: &Path) -> Arc<TargetSaveRevisionGate> {
        let mut targets = self
            .targets
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Arc::clone(
            targets
                .entry(path.to_path_buf())
                .or_insert_with(|| Arc::new(TargetSaveRevisionGate::default())),
        )
    }

    fn reserve(&self, target: PathBuf) -> ConfigSaveRevision {
        let revision = self.target(&target).reserve();
        ConfigSaveRevision { target, revision }
    }

    fn run_if_current<E>(
        &self,
        reservation: &ConfigSaveRevision,
        write: impl FnOnce() -> Result<(), E>,
    ) -> Result<bool, E> {
        self.target(&reservation.target)
            .run_if_current(reservation.revision, write)
    }
}

static SAVE_GATE: LazyLock<SaveRevisionGate> = LazyLock::new(SaveRevisionGate::default);

/// Reserve a revision for a configuration snapshot that will be saved later.
pub fn reserve_save_revision() -> Result<ConfigSaveRevision, ConfigError> {
    Ok(SAVE_GATE.reserve(config_path()?))
}

/// Persist configuration to disk, overwriting any previous contents.
///
/// Settings are written to TOML while sources are stored in SQLite.
pub fn save(config: &AppConfig) -> Result<(), ConfigError> {
    let revision = reserve_save_revision()?;
    require_current_save(save_if_revision_current(config, &revision)?)
}

fn require_current_save(saved: bool) -> Result<(), ConfigError> {
    if saved {
        Ok(())
    } else {
        Err(ConfigError::SaveSuperseded)
    }
}

/// Save a previously captured configuration only when no newer save was requested.
///
/// Returns `false` without writing when `revision` has been superseded.
pub fn save_if_revision_current(
    config: &AppConfig,
    revision: &ConfigSaveRevision,
) -> Result<bool, ConfigError> {
    SAVE_GATE.run_if_current(revision, || save_to_path(config, &revision.target))
}

/// Save configuration to a specific path, creating parent directories as needed.
pub fn save_to_path(config: &AppConfig, path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ConfigError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let settings = AppSettings::from(config);
    save_settings_to_path(&settings, path)?;
    crate::sample_sources::library::save(&crate::sample_sources::library::LibraryState {
        sources: config.sources.clone(),
    })?;
    Ok(())
}

/// Write the TOML settings file atomically to prevent partial writes on crash.
pub(super) fn save_settings_to_path(
    settings: &AppSettings,
    path: &Path,
) -> Result<(), ConfigError> {
    let data = toml::to_string_pretty(settings).map_err(|source| ConfigError::SerializeToml {
        path: path.to_path_buf(),
        source: Box::new(source),
    })?;
    atomic_write(path, data.as_bytes())
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), ConfigError> {
    atomic_write_with(path, data, publish_temp_file)
}

fn atomic_write_with(
    path: &Path,
    data: &[u8],
    publish: impl Fn(&Path, &Path, &Path) -> Result<(), ConfigError>,
) -> Result<(), ConfigError> {
    use rand::TryRngCore;
    let dir = path.parent().ok_or_else(|| ConfigError::Write {
        path: path.to_path_buf(),
        source: std::io::Error::other("config path has no parent directory"),
    })?;
    let file_name = path.file_name().ok_or_else(|| ConfigError::Write {
        path: path.to_path_buf(),
        source: std::io::Error::other("config path has no file name"),
    })?;

    let mut last_err = None;
    for _ in 0..5 {
        let mut bytes = [0u8; 6];
        rand::rngs::OsRng
            .try_fill_bytes(&mut bytes)
            .map_err(|source| ConfigError::Write {
                path: path.to_path_buf(),
                source: std::io::Error::other(format!(
                    "failed to generate temporary file suffix: {source}"
                )),
            })?;
        let suffix: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let tmp_path = dir.join(format!("{}.tmp-{}", file_name.to_string_lossy(), suffix));

        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path);

        let mut file = match file {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                last_err = Some(err);
                continue;
            }
            Err(err) => {
                return Err(ConfigError::Write {
                    path: tmp_path.clone(),
                    source: err,
                });
            }
        };

        if let Err(err) = file.write_all(data) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(ConfigError::Write {
                path: tmp_path.clone(),
                source: err,
            });
        }
        if let Err(err) = file.sync_all() {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(ConfigError::Write {
                path: tmp_path.clone(),
                source: err,
            });
        }
        drop(file);
        if let Err(err) = publish(&tmp_path, path, dir) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(err);
        }
        return Ok(());
    }

    Err(ConfigError::Write {
        path: path.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "failed to create temporary file for {}: {}",
                path.display(),
                last_err
                    .as_ref()
                    .map(|err| err.to_string())
                    .unwrap_or_else(|| "unknown error".into())
            ),
        ),
    })
}

fn publish_temp_file(temp_path: &Path, path: &Path, dir: &Path) -> Result<(), ConfigError> {
    replace_file(temp_path, path).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    sync_parent_dir(dir)
}

#[cfg(not(target_os = "windows"))]
fn replace_file(temp_path: &Path, path: &Path) -> Result<(), std::io::Error> {
    std::fs::rename(temp_path, path)
}

#[cfg(target_os = "windows")]
fn replace_file(temp_path: &Path, path: &Path) -> Result<(), std::io::Error> {
    use windows::{
        Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
        core::PCWSTR,
    };

    let temp_path = wide_path(temp_path);
    let path = wide_path(path);
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
    unsafe { MoveFileExW(PCWSTR(temp_path.as_ptr()), PCWSTR(path.as_ptr()), flags) }
        .map_err(std::io::Error::other)
}

#[cfg(target_os = "windows")]
fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn sync_parent_dir(dir: &Path) -> Result<(), ConfigError> {
    #[cfg(unix)]
    {
        let dir_handle = std::fs::File::open(dir).map_err(|source| ConfigError::Write {
            path: dir.to_path_buf(),
            source,
        })?;
        dir_handle.sync_all().map_err(|source| ConfigError::Write {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ConfigError, SaveRevisionGate, atomic_write_with, require_current_save};
    use std::{cell::Cell, path::PathBuf};

    #[test]
    fn replacement_failure_preserves_existing_settings_and_cleans_temporary_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("config.toml");
        std::fs::write(&path, b"old = true\n").unwrap();

        let result = atomic_write_with(&path, b"new = true\n", |temp_path, path, _dir| {
            assert_eq!(std::fs::read(temp_path).unwrap(), b"new = true\n");
            Err(ConfigError::Write {
                path: path.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected replacement failure",
                ),
            })
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"old = true\n");
        let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(remaining, [path.file_name().unwrap()]);
    }

    #[test]
    fn durability_failure_after_replacement_leaves_new_settings_recoverable() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("config.toml");
        std::fs::write(&path, b"old = true\n").unwrap();

        let result = atomic_write_with(&path, b"new = true\n", |temp_path, path, dir| {
            std::fs::rename(temp_path, path).unwrap();
            Err(ConfigError::Write {
                path: dir.to_path_buf(),
                source: std::io::Error::other("injected directory sync failure"),
            })
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"new = true\n");
    }

    #[test]
    fn stale_snapshot_is_skipped_after_newer_revision_is_reserved() {
        let gate = SaveRevisionGate::default();
        let target = PathBuf::from("config.toml");
        let stale = gate.reserve(target.clone());
        let current = gate.reserve(target);
        let writes = Cell::new(0);

        assert_eq!(
            gate.run_if_current(&stale, || {
                writes.set(writes.get() + 1);
                Ok::<(), ()>(())
            }),
            Ok(false)
        );
        assert_eq!(
            gate.run_if_current(&current, || {
                writes.set(writes.get() + 1);
                Ok::<(), ()>(())
            }),
            Ok(true)
        );
        assert_eq!(writes.get(), 1);
    }

    #[test]
    fn revisions_are_scoped_to_the_config_target() {
        let gate = SaveRevisionGate::default();
        let first_target = gate.reserve(PathBuf::from("first/config.toml"));
        let second_target = gate.reserve(PathBuf::from("second/config.toml"));
        let _newer_first = gate.reserve(PathBuf::from("first/config.toml"));

        assert_eq!(
            gate.run_if_current(&second_target, || Ok::<(), ()>(())),
            Ok(true)
        );
        assert_eq!(
            gate.run_if_current(&first_target, || Ok::<(), ()>(())),
            Ok(false)
        );
    }

    #[test]
    fn direct_save_reports_superseded_revision() {
        assert!(matches!(
            require_current_save(false),
            Err(ConfigError::SaveSuperseded)
        ));
    }
}
