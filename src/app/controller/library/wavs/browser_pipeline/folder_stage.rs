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
    let entries_len = controller.wav_entries_len();
    if controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint
        == Some(fingerprint)
        && controller
            .ui_cache
            .browser
            .pipeline
            .folder_accepts_by_index
            .len()
            == entries_len
    {
        return;
    }

    let accepts = if has_folder_filters {
        let mut accepts = Vec::with_capacity(entries_len);
        for index in 0..entries_len {
            let accepted = controller
                .ensure_wav_page_loaded(index)
                .ok()
                .and_then(|_| controller.wav_entries.entry(index))
                .is_some_and(|entry| {
                    crate::app::controller::library::source_folders::folder_filter_accepts(
                        &entry.relative_path,
                        folder_selection,
                        folder_negated,
                        file_scope_mode,
                    )
                });
            accepts.push(accepted);
        }
        accepts
    } else {
        vec![true; entries_len]
    };

    controller.ui_cache.browser.pipeline.folder_accepts_by_index = accepts;
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint = Some(fingerprint);
}

/// Return cached folder-filter acceptance for an absolute wav-entry index.
pub(super) fn folder_accepts(controller: &AppController, index: usize) -> bool {
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_by_index
        .get(index)
        .copied()
        .unwrap_or(false)
}
