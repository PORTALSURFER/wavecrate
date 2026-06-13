use super::super::jobs::{
    JobMessage, MetadataMutationResult, SourceDbMaintenanceOutcome, SourceDbMaintenanceRefresh,
    SourceDbMaintenanceResult, SourceHydrationKind, SourceHydrationResult, SourceHydrationSnapshot,
};
use super::super::library::source_folders::with_folder_projection_async_enabled_for_tests;
use super::super::library::sources::hydration::with_source_hydration_async_enabled_for_tests;
use super::super::library::wavs::with_browser_async_pipeline_enabled_for_tests;
use super::super::test_support::sample_entry;
use super::super::*;
use super::common::visible_indices;
use crate::app::controller::state::runtime::PendingMetadataMutation;
use crate::app::state::FolderPaneId;
use crate::sample_sources::Rating;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn build_controller_with_sources(names: &[&str]) -> (AppController, Vec<SampleSource>) {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir);
    let mut sources = Vec::new();
    for name in names {
        let source_root = root.join(name);
        std::fs::create_dir_all(&source_root).unwrap();
        let source = SampleSource::new(source_root);
        controller.cache_db(&source).unwrap();
        controller.library.sources.push(source.clone());
        sources.push(source);
    }
    controller.assign_source_to_folder_pane(FolderPaneId::Upper, Some(sources[0].id.clone()));
    controller.selection_state.ctx.selected_source = Some(sources[0].id.clone());
    controller
        .selection_state
        .ctx
        .last_selected_browsable_source = Some(sources[0].id.clone());
    controller.refresh_sources_ui();
    (controller, sources)
}

fn cache_source_entries(
    controller: &mut AppController,
    source: &SampleSource,
    entries: Vec<WavEntry>,
) {
    let total = entries.len();
    controller.cache.wav.insert_page(
        source.id.clone(),
        total,
        controller.wav_entries.page_size,
        0,
        entries,
    );
}

fn upsert_source_db_entry(controller: &mut AppController, source: &SampleSource, entry: &WavEntry) {
    let absolute_path = source.root.join(&entry.relative_path);
    if let Some(parent) = entry.relative_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(source.root.join(parent)).unwrap();
    }
    std::fs::write(&absolute_path, b"fixture").unwrap();
    let db = controller.database_for(source).unwrap();
    db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
        .unwrap();
    db.set_tag(&entry.relative_path, entry.tag).unwrap();
    db.set_looped(&entry.relative_path, entry.looped).unwrap();
    db.set_locked(&entry.relative_path, entry.locked).unwrap();
}

fn hydration_result(
    controller: &AppController,
    source: &SampleSource,
    request_id: u64,
    pane: FolderPaneId,
    kind: SourceHydrationKind,
    entries: Vec<WavEntry>,
    from_cache: bool,
) -> SourceHydrationResult {
    let available_folders = entries
        .iter()
        .filter_map(|entry| entry.relative_path.parent())
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .collect::<BTreeSet<_>>();
    let folder_tree =
        crate::app::controller::library::source_folders::FolderTreeSnapshot::from_available(
            &available_folders,
        );
    let path_lookup = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            (
                PathBuf::from(entry.relative_path.to_string_lossy().replace('\\', "/")),
                index,
            )
        })
        .collect::<HashMap<_, _>>();
    SourceHydrationResult {
        request_id,
        pane,
        kind,
        source_id: source.id.clone(),
        elapsed: std::time::Duration::from_millis(5),
        result: Ok(SourceHydrationSnapshot {
            entries,
            total: path_lookup.len(),
            page_size: controller.wav_entries.page_size,
            path_lookup,
            available_folders,
            folder_tree,
            feature_cache: None,
            from_cache,
            deferred_follow_up_work: false,
        }),
    }
}

fn maintenance_result(
    source: &SampleSource,
    refresh: SourceDbMaintenanceRefresh,
    orphan_rows_removed: usize,
) -> JobMessage {
    JobMessage::SourceDbMaintenanceFinished(SourceDbMaintenanceResult {
        outcomes: vec![SourceDbMaintenanceOutcome {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            skipped: false,
            deferred_due_to_file_op: false,
            orphan_rows_removed,
            refresh,
            error: None,
        }],
    })
}

fn assert_no_analysis_message(controller: &mut AppController) {
    loop {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message)) => {
                panic!("unexpected analysis message: {message:?}");
            }
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(err) => panic!("unexpected receive error: {err:?}"),
        }
    }
}

mod active_hydration;
mod file_op_gating;
mod inactive_projection;
mod maintenance_reconcile;
mod passive_status;
mod startup_deferral;
