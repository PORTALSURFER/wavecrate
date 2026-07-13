use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use serde::{Deserialize, Serialize};

use super::{
    FolderEntry, SourceEntry, collections::MissingCollectionSnapshot, scan::FolderScanResult,
};

const SOURCE_SCAN_CACHE_FILE_NAME: &str = "source-scan-cache.json";
const SOURCE_SCAN_CACHE_VERSION: u32 = 2;

#[derive(Default)]
struct CacheSaveRevisionGate {
    revision: AtomicU64,
    lock: Mutex<()>,
}

impl CacheSaveRevisionGate {
    fn reserve(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::AcqRel).wrapping_add(1)
    }

    fn run_if_current(
        &self,
        revision: u64,
        write: impl FnOnce() -> Result<(), String>,
    ) -> Result<bool, String> {
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

static CACHE_SAVE_GATE: LazyLock<CacheSaveRevisionGate> =
    LazyLock::new(CacheSaveRevisionGate::default);

pub(in crate::native_app) fn reserve_source_scan_cache_revision() -> u64 {
    CACHE_SAVE_GATE.reserve()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct SourceScanCache {
    version: u32,
    sources: Vec<CachedSourceScan>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CachedSourceScan {
    source_id: String,
    root: PathBuf,
    root_folder: FolderEntry,
    #[serde(default)]
    missing_collection_snapshot: MissingCollectionSnapshot,
}

impl SourceScanCache {
    fn new(sources: Vec<CachedSourceScan>) -> Self {
        let mut cache = Self {
            version: SOURCE_SCAN_CACHE_VERSION,
            sources,
        };
        cache.prune_apple_double_sidecars();
        cache
    }

    #[cfg(test)]
    pub(super) fn folder_for_source(&self, source_id: &str, root: &Path) -> Option<FolderEntry> {
        self.source_snapshot_for_source(source_id, root)
            .map(|snapshot| snapshot.root_folder)
    }

    pub(super) fn source_snapshot_for_source(
        &self,
        source_id: &str,
        root: &Path,
    ) -> Option<CachedSourceSnapshot> {
        if self.version != SOURCE_SCAN_CACHE_VERSION {
            return None;
        }
        self.sources
            .iter()
            .find(|source| source.source_id == source_id && source.root == root)
            .map(|source| {
                let mut folder = source.root_folder.clone();
                prune_folder_apple_double_sidecars(&mut folder);
                CachedSourceSnapshot {
                    root_folder: folder,
                    missing_collection_snapshot: source.missing_collection_snapshot.clone(),
                }
            })
    }

    fn prune_apple_double_sidecars(&mut self) {
        for source in &mut self.sources {
            prune_folder_apple_double_sidecars(&mut source.root_folder);
        }
    }
}

pub(super) struct CachedSourceSnapshot {
    pub(super) root_folder: FolderEntry,
    pub(super) missing_collection_snapshot: MissingCollectionSnapshot,
}

pub(super) fn load_source_scan_cache() -> Result<SourceScanCache, String> {
    load_source_scan_cache_from_path(&source_scan_cache_path()?)
}

pub(super) fn save_source_scan_cache(sources: &[SourceEntry]) -> Result<(), String> {
    let path = source_scan_cache_path()?;
    let revision = reserve_source_scan_cache_revision();
    if CACHE_SAVE_GATE
        .run_if_current(revision, || save_source_scan_cache_to_path(&path, sources))?
    {
        Ok(())
    } else {
        Err(String::from(
            "source scan cache save was superseded by a newer snapshot",
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanCacheUpdate {
    source_id: String,
    source: Option<CachedSourceScan>,
}

pub(in crate::native_app) fn prepare_folder_scan_cache_update(
    result: &FolderScanResult,
) -> FolderScanCacheUpdate {
    FolderScanCacheUpdate {
        source_id: result.source_id.clone(),
        source: result.source_root_available.then(|| CachedSourceScan {
            source_id: result.source_id.clone(),
            root: PathBuf::from(&result.folder.id),
            root_folder: result.folder.clone(),
            missing_collection_snapshot: result.missing_collection_snapshot.clone(),
        }),
    }
}

pub(in crate::native_app) fn apply_folder_scan_cache_update(
    update: FolderScanCacheUpdate,
    revision: u64,
) -> Result<(), String> {
    let path = source_scan_cache_path()?;
    CACHE_SAVE_GATE
        .run_if_current(revision, || {
            apply_folder_scan_cache_update_to_path(&path, update)
        })
        .map(|_| ())
}

fn apply_folder_scan_cache_update_to_path(
    path: &Path,
    update: FolderScanCacheUpdate,
) -> Result<(), String> {
    let Some(source) = update.source else {
        return Ok(());
    };
    let mut cache = load_source_scan_cache_from_path(path).unwrap_or_default();
    cache.version = SOURCE_SCAN_CACHE_VERSION;
    cache
        .sources
        .retain(|source| source.source_id != update.source_id);
    cache.sources.push(source);
    cache
        .sources
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    save_source_scan_cache_value_to_path(path, &cache)
}

fn source_scan_cache_path() -> Result<PathBuf, String> {
    wavecrate::app_dirs::app_root_dir()
        .map(|root| root.join(SOURCE_SCAN_CACHE_FILE_NAME))
        .map_err(|err| format!("resolve source scan cache path: {err}"))
}

fn load_source_scan_cache_from_path(path: &Path) -> Result<SourceScanCache, String> {
    if !path.exists() {
        return Ok(SourceScanCache::default());
    }
    let text = fs::read_to_string(path)
        .map_err(|err| format!("read source scan cache {}: {err}", path.display()))?;
    let cache = serde_json::from_str::<SourceScanCache>(&text)
        .map_err(|err| format!("parse source scan cache {}: {err}", path.display()))?;
    if cache.version == SOURCE_SCAN_CACHE_VERSION {
        let mut cache = cache;
        cache.prune_apple_double_sidecars();
        Ok(cache)
    } else {
        Ok(SourceScanCache::default())
    }
}

fn save_source_scan_cache_to_path(path: &Path, sources: &[SourceEntry]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "create source scan cache directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let cache = SourceScanCache::new(
        sources
            .iter()
            .filter(|source| !source.is_default_assets_source())
            .filter_map(|source| {
                source
                    .root_folder
                    .as_ref()
                    .map(|root_folder| CachedSourceScan {
                        source_id: source.id.clone(),
                        root: source.root.clone(),
                        root_folder: root_folder.clone(),
                        missing_collection_snapshot: source.missing_collection_snapshot.clone(),
                    })
            })
            .collect(),
    );
    save_source_scan_cache_value_to_path(path, &cache)
}

fn save_source_scan_cache_value_to_path(
    path: &Path,
    cache: &SourceScanCache,
) -> Result<(), String> {
    let bytes =
        serde_json::to_vec(cache).map_err(|err| format!("serialize source scan cache: {err}"))?;
    atomic_write(path, &bytes)
}

fn prune_folder_apple_double_sidecars(folder: &mut FolderEntry) {
    folder.files.retain(|file| {
        !wavecrate_library::sample_sources::is_apple_double_sidecar(Path::new(&file.name))
    });
    for child in &mut folder.children {
        prune_folder_apple_double_sidecars(child);
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("source scan cache path has no parent: {}", path.display()))?;
    let file_name = path.file_name().ok_or_else(|| {
        format!(
            "source scan cache path has no file name: {}",
            path.display()
        )
    })?;
    let tmp_path = parent.join(format!("{}.tmp", file_name.to_string_lossy()));
    fs::write(&tmp_path, bytes)
        .map_err(|err| format!("write source scan cache {}: {err}", tmp_path.display()))?;
    replace_file(&tmp_path, path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        format!("replace source scan cache {}: {err}", path.display())
    })
}

fn replace_file(temp_path: &Path, path: &Path) -> Result<(), std::io::Error> {
    match fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            #[cfg(target_os = "windows")]
            if err.kind() == std::io::ErrorKind::AlreadyExists
                || err.kind() == std::io::ErrorKind::PermissionDenied
            {
                if let Err(inner) = fs::remove_file(path)
                    && inner.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(inner);
                }
                fs::rename(temp_path, path)?;
                return Ok(());
            }
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_revision_gate_skips_superseded_maintenance_write() {
        let gate = CacheSaveRevisionGate::default();
        let stale = gate.reserve();
        let _current = gate.reserve();
        let mut wrote = false;

        let applied = gate
            .run_if_current(stale, || {
                wrote = true;
                Ok(())
            })
            .unwrap();

        assert!(!applied);
        assert!(!wrote);
    }
    use crate::native_app::sample_library::folder_browser::state_types::SourceAvailability;
    use crate::native_app::sample_library::folder_browser::{FolderEntry, model::FileEntry};
    use wavecrate::sample_sources::{Rating, SampleCollection};

    #[test]
    fn source_scan_cache_round_trips_loaded_sources() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("source");
        let source = SourceEntry {
            id: String::from("source-id"),
            label: String::from("Source"),
            root: root.clone(),
            database_root: root.clone(),
            role: wavecrate::sample_sources::SourceRole::Normal,
            availability: SourceAvailability::Available,
            metadata_storage: wavecrate::sample_sources::SourceMetadataStorage::SourceFolder,
            primary_import_folder: wavecrate::sample_sources::default_primary_import_folder(),
            root_folder: Some(FolderEntry {
                id: root.display().to_string(),
                name: String::from("source"),
                children: Vec::new(),
                files: vec![FileEntry {
                    id: root.join("kick.wav").display().to_string(),
                    name: String::from("kick.wav"),
                    stem: String::from("kick"),
                    extension: String::from("wav"),
                    kind: String::from("Audio"),
                    size: String::from("8 B"),
                    size_bytes: 8,
                    modified: String::from("now"),
                    modified_rank: 1,
                    rating: Rating::KEEP_1,
                    rating_locked: true,
                    last_curated_at: None,
                    collection: SampleCollection::new(0),
                    collections: SampleCollection::new(0).into_iter().collect(),
                }],
            }),
            parked_tree_loaded: false,
            missing_collection_snapshot: MissingCollectionSnapshot::default(),
            loading_task: None,
        };
        let path = temp.path().join(SOURCE_SCAN_CACHE_FILE_NAME);

        save_source_scan_cache_to_path(&path, &[source]).expect("save cache");
        let cache = load_source_scan_cache_from_path(&path).expect("load cache");
        let folder = cache
            .folder_for_source("source-id", &root)
            .expect("cached folder");

        assert_eq!(folder.files[0].name, "kick.wav");
        assert_eq!(folder.files[0].rating, Rating::KEEP_1);
        assert_eq!(folder.files[0].collection, SampleCollection::new(0));
        assert_eq!(
            folder.files[0].collections,
            SampleCollection::new(0).into_iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn source_scan_cache_prunes_appledouble_sidecars_on_load() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("source");
        let path = temp.path().join(SOURCE_SCAN_CACHE_FILE_NAME);
        let cache = SourceScanCache {
            version: SOURCE_SCAN_CACHE_VERSION,
            sources: vec![CachedSourceScan {
                source_id: String::from("source-id"),
                root: root.clone(),
                root_folder: FolderEntry {
                    id: root.display().to_string(),
                    name: String::from("source"),
                    children: vec![FolderEntry {
                        id: root.join("drums").display().to_string(),
                        name: String::from("drums"),
                        children: Vec::new(),
                        files: vec![
                            file_for_cache_test(&root.join("drums/kick.wav")),
                            file_for_cache_test(&root.join("drums/._kick.wav")),
                        ],
                    }],
                    files: vec![
                        file_for_cache_test(&root.join("kick.wav")),
                        file_for_cache_test(&root.join("._kick.wav")),
                    ],
                },
                missing_collection_snapshot: MissingCollectionSnapshot::default(),
            }],
        };
        fs::write(&path, serde_json::to_vec(&cache).expect("serialize cache"))
            .expect("write cache");

        let loaded = load_source_scan_cache_from_path(&path).expect("load cache");
        let folder = loaded
            .folder_for_source("source-id", &root)
            .expect("cached folder");

        assert_eq!(
            folder
                .files
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );
        assert_eq!(
            folder.children[0]
                .files
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["kick.wav"]
        );
    }

    #[test]
    fn incremental_update_writes_current_cache_version_for_new_profiles() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(SOURCE_SCAN_CACHE_FILE_NAME);
        let root = temp.path().join("source");

        apply_folder_scan_cache_update_to_path(
            &path,
            FolderScanCacheUpdate {
                source_id: String::from("source-id"),
                source: Some(cached_source_for_test("source-id", &root)),
            },
        )
        .expect("apply cache update");

        let cache = load_source_scan_cache_from_path(&path).expect("load updated cache");
        assert_eq!(cache.version, SOURCE_SCAN_CACHE_VERSION);
        assert!(cache.folder_for_source("source-id", &root).is_some());
    }

    #[test]
    fn incremental_update_preserves_offline_source_snapshot() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(SOURCE_SCAN_CACHE_FILE_NAME);
        let root = temp.path().join("source");
        let cache = SourceScanCache::new(vec![cached_source_for_test("source-id", &root)]);
        save_source_scan_cache_value_to_path(&path, &cache).expect("seed cache");
        let before = fs::read(&path).expect("read seeded cache");

        apply_folder_scan_cache_update_to_path(
            &path,
            FolderScanCacheUpdate {
                source_id: String::from("source-id"),
                source: None,
            },
        )
        .expect("preserve offline cache");

        assert_eq!(fs::read(&path).expect("read preserved cache"), before);
    }

    #[test]
    fn incremental_update_replaces_corrupt_cache() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(SOURCE_SCAN_CACHE_FILE_NAME);
        let root = temp.path().join("source");
        fs::write(&path, b"{not-json").expect("seed corrupt cache");

        apply_folder_scan_cache_update_to_path(
            &path,
            FolderScanCacheUpdate {
                source_id: String::from("source-id"),
                source: Some(cached_source_for_test("source-id", &root)),
            },
        )
        .expect("repair cache");

        let cache = load_source_scan_cache_from_path(&path).expect("load repaired cache");
        assert!(cache.folder_for_source("source-id", &root).is_some());
    }

    fn cached_source_for_test(source_id: &str, root: &Path) -> CachedSourceScan {
        CachedSourceScan {
            source_id: source_id.to_owned(),
            root: root.to_path_buf(),
            root_folder: FolderEntry {
                id: root.display().to_string(),
                name: String::from("source"),
                children: Vec::new(),
                files: vec![file_for_cache_test(&root.join("kick.wav"))],
            },
            missing_collection_snapshot: MissingCollectionSnapshot::default(),
        }
    }

    fn file_for_cache_test(path: &Path) -> FileEntry {
        let name = path
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .to_string();
        FileEntry {
            id: path.display().to_string(),
            stem: path
                .file_stem()
                .expect("file stem")
                .to_string_lossy()
                .to_string(),
            extension: String::from("wav"),
            name,
            kind: String::from("Audio"),
            size: String::from("8 B"),
            size_bytes: 8,
            modified: String::from("now"),
            modified_rank: 1,
            rating: Rating::NEUTRAL,
            rating_locked: false,
            last_curated_at: None,
            collection: None,
            collections: Vec::new(),
        }
    }
}
