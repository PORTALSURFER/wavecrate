use super::*;
use std::path::Path;

use super::audio::{invalidate_new_entry_audio, invalidate_old_entry_audio};
use super::browser_caches::remap_path_scoped_browser_caches;
use super::missing::remap_missing_file_state;
use super::projection::{apply_path_change_projection, apply_same_path_metadata_projection};
use super::selection_paths::update_selection_paths;
use super::source_entries::{update_path_changed_entry, update_same_path_entry};

/// Update all cached structures after a file path or metadata change.
pub(crate) fn update_cached_entry(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_entry: WavEntry,
) {
    let plan = CachedEntryUpdatePlan::new(controller, source, old_path, &new_entry);
    update_selection_paths(controller, source, plan.old_path, plan.new_path);
    invalidate_old_entry_audio(controller, &source.id, plan.old_path);
    remap_path_scoped_browser_caches(controller, &source.id, plan.old_path, plan.new_path);
    remap_missing_file_state(controller, &source.id, plan.old_path, &new_entry);
    if !plan.path_changed {
        let mutation = update_same_path_entry(controller, &source.id, plan.old_path, &new_entry);
        apply_same_path_metadata_projection(
            controller,
            source,
            plan.old_path,
            &new_entry,
            mutation,
        );
        return;
    }

    rewrite_source_db_entry_if_present(controller, source, plan.old_path, &new_entry);
    let mutation = update_path_changed_entry(controller, &source.id, plan.old_path, &new_entry);
    apply_path_change_projection(
        controller,
        source,
        plan.old_path,
        &new_entry,
        plan.keep_visible_order,
        mutation,
    );
    invalidate_new_entry_audio(controller, &source.id, &new_entry.relative_path);
}

#[derive(Clone, Copy, Debug)]
struct CachedEntryUpdatePlan<'a> {
    old_path: &'a Path,
    new_path: &'a Path,
    path_changed: bool,
    keep_visible_order: bool,
}

impl<'a> CachedEntryUpdatePlan<'a> {
    fn new(
        controller: &AppController,
        _source: &SampleSource,
        old_path: &'a Path,
        new_entry: &'a WavEntry,
    ) -> Self {
        let new_path = new_entry.relative_path.as_path();
        let path_changed = old_path != new_path;
        Self {
            old_path,
            new_path,
            path_changed,
            keep_visible_order: path_changed
                && same_parent(old_path, new_path)
                && controller.active_search_query().is_none()
                && controller.ui.browser.search.similar_query.is_none(),
        }
    }
}

fn rewrite_source_db_entry_if_present(
    controller: &mut AppController,
    source: &SampleSource,
    old_path: &Path,
    new_entry: &WavEntry,
) {
    if let Ok(db) = controller.database_for(source)
        && matches!(db.index_for_path(old_path), Ok(Some(_)))
    {
        let _ = controller.rewrite_db_entry_for_source(
            source,
            old_path,
            &new_entry.relative_path,
            new_entry.file_size,
            new_entry.modified_ns,
            new_entry.tag,
        );
    }
}

fn same_parent(left: &Path, right: &Path) -> bool {
    left.parent().unwrap_or_else(|| Path::new(""))
        == right.parent().unwrap_or_else(|| Path::new(""))
}
