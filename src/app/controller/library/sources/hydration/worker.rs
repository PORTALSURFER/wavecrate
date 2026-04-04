use super::super::super::*;
use super::super::hydration_telemetry;
use crate::app::controller::jobs::{
    SourceHydrationJob, SourceHydrationResult, SourceHydrationSnapshot,
};
use crate::app::controller::library::source_folders::FolderTreeSnapshot;
use crate::app::controller::library::wav_entries_loader;
use crate::app::controller::library::wavs::build_feature_cache_for_paths;
use std::collections::{BTreeSet, HashMap};
use std::path::{Component, PathBuf};
use std::time::Instant;

#[cfg(test)]
use std::{cell::Cell, thread_local};

pub(super) fn run_source_hydration(job: SourceHydrationJob) -> SourceHydrationResult {
    let start = Instant::now();
    let result = build_source_hydration_snapshot(&job);
    hydration_telemetry::record_source_hydration_worker(result.is_ok(), start.elapsed());
    SourceHydrationResult {
        request_id: job.request_id,
        pane: job.pane,
        kind: job.kind,
        source_id: job.source_id,
        elapsed: start.elapsed(),
        result,
    }
}

fn build_source_hydration_snapshot(
    job: &SourceHydrationJob,
) -> Result<SourceHydrationSnapshot, LoadEntriesError> {
    let deferred_follow_up_work = job.defer_startup_follow_up_work
        && job.kind == crate::app::controller::jobs::SourceHydrationKind::ActiveSelection;
    let (entries, total, page_size, from_cache) = if let Some(entries) = job.cached_page.clone() {
        (
            entries,
            job.cached_total.unwrap_or_default(),
            job.cached_page_size.unwrap_or(job.page_size),
            true,
        )
    } else {
        let load_job = WavLoadJob {
            source_id: job.source_id.clone(),
            root: job.source_root.clone(),
            page_size: job.page_size,
        };
        let (result, total) = if deferred_follow_up_work {
            wav_entries_loader::load_entries_startup_fast_path(&load_job)
        } else {
            wav_entries_loader::load_entries(&load_job)
        };
        (result?, total, job.page_size, false)
    };
    let entry_maps =
        build_hydration_entry_maps(&job.source_root, &entries, !deferred_follow_up_work);
    let feature_cache = if deferred_follow_up_work {
        None
    } else if job.kind == crate::app::controller::jobs::SourceHydrationKind::ActiveSelection {
        Some(build_feature_cache_for_paths(
            &job.source_id,
            &job.source_root,
            &entry_maps.entry_paths,
            &[],
        )?)
    } else {
        None
    };
    Ok(SourceHydrationSnapshot {
        folder_tree: FolderTreeSnapshot::from_available(&entry_maps.available_folders),
        available_folders: entry_maps.available_folders,
        feature_cache,
        path_lookup: entry_maps.path_lookup,
        entries,
        total,
        page_size,
        from_cache,
        deferred_follow_up_work,
    })
}

struct HydrationEntryMaps {
    path_lookup: HashMap<PathBuf, usize>,
    available_folders: BTreeSet<PathBuf>,
    entry_paths: Vec<PathBuf>,
}

fn build_hydration_entry_maps(
    source_root: &std::path::Path,
    entries: &[WavEntry],
    include_available_folders: bool,
) -> HydrationEntryMaps {
    let mut path_lookup = HashMap::with_capacity(entries.len());
    let mut available_folders = BTreeSet::new();
    let mut entry_paths = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        path_lookup.insert(normalized_lookup_path(&entry.relative_path), index);
        if include_available_folders {
            let mut current = entry.relative_path.parent();
            while let Some(path) = current {
                if !path.as_os_str().is_empty() {
                    available_folders.insert(path.to_path_buf());
                }
                current = path.parent();
            }
        }
        entry_paths.push(entry.relative_path.clone());
    }
    if include_available_folders {
        available_folders.retain(|path| source_root.join(path).is_dir());
    }
    HydrationEntryMaps {
        path_lookup,
        available_folders,
        entry_paths,
    }
}

fn normalized_lookup_path(path: &std::path::Path) -> PathBuf {
    let mut normalized = String::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                if !normalized.is_empty() {
                    normalized.push('/');
                }
                normalized.push_str(&part.to_string_lossy());
            }
            Component::ParentDir => {
                if !normalized.is_empty() {
                    normalized.push('/');
                }
                normalized.push_str("..");
            }
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    PathBuf::from(normalized)
}

pub(super) fn source_hydration_async_enabled() -> bool {
    #[cfg(test)]
    {
        source_hydration_async_override_for_tests().unwrap_or(false)
    }
    #[cfg(not(test))]
    {
        true
    }
}

#[cfg(test)]
thread_local! {
    static SOURCE_HYDRATION_ASYNC_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

#[cfg(test)]
fn source_hydration_async_override_for_tests() -> Option<bool> {
    SOURCE_HYDRATION_ASYNC_OVERRIDE.with(|value| value.get())
}

#[cfg(test)]
pub(crate) fn with_source_hydration_async_enabled_for_tests<T>(
    enabled: bool,
    run: impl FnOnce() -> T,
) -> T {
    struct Reset<'a> {
        cell: &'a Cell<Option<bool>>,
        previous: Option<bool>,
    }

    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.cell.set(self.previous);
        }
    }

    SOURCE_HYDRATION_ASYNC_OVERRIDE.with(|value| {
        let previous = value.replace(Some(enabled));
        let _reset = Reset {
            cell: value,
            previous,
        };
        run()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::Rating;
    use std::path::{Path, PathBuf};

    #[test]
    fn hydration_entry_maps_build_lookup_and_folders_in_one_pass() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("kits").join("drums")).expect("create drums");
        std::fs::create_dir_all(temp.path().join("kits").join("perc")).expect("create perc");
        let entries = vec![
            WavEntry {
                relative_path: PathBuf::from(r"kits\drums\kick.wav"),
                file_size: 0,
                modified_ns: 0,
                content_hash: None,
                tag: Rating::NEUTRAL,
                looped: false,
                locked: false,
                missing: false,
                last_played_at: None,
            },
            WavEntry {
                relative_path: PathBuf::from("kits/perc/snare.wav"),
                file_size: 0,
                modified_ns: 0,
                content_hash: None,
                tag: Rating::NEUTRAL,
                looped: false,
                locked: false,
                missing: false,
                last_played_at: None,
            },
        ];

        let maps = build_hydration_entry_maps(temp.path(), &entries, true);

        assert_eq!(
            maps.path_lookup.get(Path::new("kits/drums/kick.wav")),
            Some(&0)
        );
        assert_eq!(
            maps.path_lookup.get(Path::new("kits/perc/snare.wav")),
            Some(&1)
        );
        assert!(maps.available_folders.contains(Path::new("kits")));
        assert!(maps.available_folders.contains(Path::new("kits/drums")));
        assert!(maps.available_folders.contains(Path::new("kits/perc")));
    }

    #[test]
    fn hydration_entry_maps_skip_folder_derivation_for_deferred_follow_up_work() {
        let temp = tempfile::tempdir().expect("tempdir");
        let entries = vec![WavEntry {
            relative_path: PathBuf::from("kits/drums/kick.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: None,
            tag: Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        }];

        let maps = build_hydration_entry_maps(temp.path(), &entries, false);

        assert!(maps.available_folders.is_empty());
        assert_eq!(maps.entry_paths, vec![PathBuf::from("kits/drums/kick.wav")]);
        assert_eq!(
            maps.path_lookup.get(Path::new("kits/drums/kick.wav")),
            Some(&0)
        );
    }

    #[test]
    fn hydration_entry_maps_filter_missing_folder_ancestors_once_per_unique_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("kits")).expect("create kits");
        let entries = vec![WavEntry {
            relative_path: PathBuf::from("kits/drums/kick.wav"),
            file_size: 0,
            modified_ns: 0,
            content_hash: None,
            tag: Rating::NEUTRAL,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        }];

        let maps = build_hydration_entry_maps(temp.path(), &entries, true);

        assert!(maps.available_folders.contains(Path::new("kits")));
        assert!(!maps.available_folders.contains(Path::new("kits/drums")));
    }
}
