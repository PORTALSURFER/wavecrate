use super::super::super::*;
use super::super::hydration_telemetry;
use crate::app::controller::jobs::{
    SourceHydrationJob, SourceHydrationResult, SourceHydrationSnapshot,
};
use crate::app::controller::library::source_folders::FolderTreeSnapshot;
use crate::app::controller::library::wav_entries_loader;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
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
    let (entries, total, page_size, from_cache) = if let Some(entries) = job.cached_page.clone() {
        (
            entries,
            job.cached_total.unwrap_or_default(),
            job.cached_page_size.unwrap_or(job.page_size),
            true,
        )
    } else {
        let (result, total) = wav_entries_loader::load_entries(&WavLoadJob {
            source_id: job.source_id.clone(),
            root: job.source_root.clone(),
            page_size: job.page_size,
        });
        (result?, total, job.page_size, false)
    };
    let available_folders = derive_available_folders(&job.source_root, &entries);
    Ok(SourceHydrationSnapshot {
        folder_tree: FolderTreeSnapshot::from_available(&available_folders),
        available_folders,
        path_lookup: build_path_lookup(&entries),
        entries,
        total,
        page_size,
        from_cache,
    })
}

fn build_path_lookup(entries: &[WavEntry]) -> HashMap<PathBuf, usize> {
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            (
                PathBuf::from(entry.relative_path.to_string_lossy().replace('\\', "/")),
                index,
            )
        })
        .collect()
}

fn derive_available_folders(source_root: &Path, entries: &[WavEntry]) -> BTreeSet<PathBuf> {
    let mut folders = BTreeSet::new();
    for entry in entries {
        let mut current = entry.relative_path.parent();
        while let Some(path) = current {
            if !path.as_os_str().is_empty() && source_root.join(path).is_dir() {
                folders.insert(path.to_path_buf());
            }
            current = path.parent();
        }
    }
    folders
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
