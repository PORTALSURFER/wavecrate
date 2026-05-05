use super::*;

/// Ensure folder-filter acceptance values are cached for the current base snapshot.
pub(super) fn ensure_folder_acceptance_stage(
    controller: &mut AppController,
    folder_selection: Option<&std::collections::BTreeSet<std::path::PathBuf>>,
    folder_negated: Option<&std::collections::BTreeSet<std::path::PathBuf>>,
    file_scope_mode: crate::app::state::FolderFileScopeMode,
    folder_hash: u64,
    has_folder_filters: bool,
) {
    let base_fingerprint_hash =
        helpers::hash_value(&controller.ui_cache.browser.pipeline.base_fingerprint);
    let fingerprint = helpers::hash_value(&(base_fingerprint_hash, folder_hash));
    let entries_len = controller.ui_cache.browser.pipeline.compact_entries.len();
    if controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint
        == Some(fingerprint)
        && controller.ui_cache.browser.pipeline.folder_accepts_active == has_folder_filters
        && (!has_folder_filters
            || controller
                .ui_cache
                .browser
                .pipeline
                .folder_accepts_by_index
                .len()
                == entries_len)
    {
        return;
    }

    if has_folder_filters {
        let mut accepts = Vec::with_capacity(entries_len);
        let mut accepted_rows = Vec::new();
        for entry in &controller.ui_cache.browser.pipeline.compact_entries {
            let accepted = crate::app::controller::library::source_folders::folder_filter_accepts(
                &entry.relative_path,
                folder_selection,
                folder_negated,
                file_scope_mode,
            );
            accepts.push(accepted);
        }
        accepted_rows.reserve(accepts.iter().filter(|accepted| **accepted).count());
        for (index, accepted) in accepts.iter().copied().enumerate() {
            if accepted {
                accepted_rows.push(index);
            }
        }
        controller.ui_cache.browser.pipeline.folder_accepts_by_index = accepts;
        controller.ui_cache.browser.pipeline.folder_filtered_rows = accepted_rows;
    } else {
        controller
            .ui_cache
            .browser
            .pipeline
            .folder_accepts_by_index
            .clear();
        controller
            .ui_cache
            .browser
            .pipeline
            .folder_filtered_rows
            .clear();
    }

    controller.ui_cache.browser.pipeline.folder_accepts_active = has_folder_filters;
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint = Some(fingerprint);
}

/// Return cached folder-filter acceptance for an absolute wav-entry index.
pub(super) fn folder_accepts(controller: &AppController, index: usize) -> bool {
    if !controller.ui_cache.browser.pipeline.folder_accepts_active {
        return true;
    }
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_by_index
        .get(index)
        .copied()
        .unwrap_or(false)
}
