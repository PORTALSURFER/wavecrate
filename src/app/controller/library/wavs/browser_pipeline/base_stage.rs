use super::*;

/// Ensure stage-A cached base rows and triage partitions are current.
pub(super) fn ensure_base_stage(controller: &mut AppController) {
    let source_id = controller.selection_state.ctx.selected_source.clone();
    let source_revision = controller
        .current_source()
        .and_then(|source| controller.database_for(&source).ok())
        .and_then(|db| db.get_revision().ok());
    let fingerprint = BaseStageFingerprint {
        source_id,
        source_revision,
        entries_len: controller.wav_entries_len(),
    };
    if controller
        .ui_cache
        .browser
        .pipeline
        .base_fingerprint
        .as_ref()
        == Some(&fingerprint)
    {
        return;
    }

    controller.ui_cache.browser.pipeline.compact_entries = load_compact_entries(controller);
    let compact_entries = &controller.ui_cache.browser.pipeline.compact_entries;
    let mut base_rows = Vec::with_capacity(compact_entries.len());
    let mut entry_paths = Vec::with_capacity(compact_entries.len());
    let mut trash_rows = Vec::new();
    let mut neutral_rows = Vec::new();
    let mut keep_rows = Vec::new();
    for (index, entry) in compact_entries.iter().enumerate() {
        base_rows.push(index);
        entry_paths.push(entry.relative_path.clone());
        if entry.tag.is_trash() {
            trash_rows.push(index);
        } else if entry.tag.is_keep() {
            keep_rows.push(index);
        } else {
            neutral_rows.push(index);
        }
    }
    let feature_cache_key =
        crate::app::controller::library::wavs::feature_cache::feature_cache_key_for_paths(
            &entry_paths,
        );
    controller.ui_cache.browser.pipeline.feature_cache_snapshot =
        Some(super::BrowserFeatureCacheSnapshot {
            key: feature_cache_key,
            entry_paths: entry_paths.into(),
        });
    controller.ui_cache.browser.pipeline.base_rows = base_rows;
    controller.ui_cache.browser.pipeline.trash_rows = trash_rows;
    controller.ui_cache.browser.pipeline.neutral_rows = neutral_rows;
    controller.ui_cache.browser.pipeline.keep_rows = keep_rows;
    controller.ui_cache.browser.pipeline.base_fingerprint = Some(fingerprint);
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_fingerprint = None;
    controller
        .ui_cache
        .browser
        .pipeline
        .folder_accepts_by_index
        .clear();
    controller.ui_cache.browser.pipeline.filtered_fingerprint = None;
    controller.ui_cache.browser.pipeline.scored_fingerprint = None;
    controller.ui_cache.browser.pipeline.sorted_fingerprint = None;
}

fn load_compact_entries(controller: &mut AppController) -> Vec<super::CompactBrowserEntry> {
    if let Some(entries) = compact_entries_from_loaded_pages(controller) {
        return entries;
    }
    let Some(source) = controller.current_source() else {
        return Vec::new();
    };
    let Ok(db) = controller.database_for(&source) else {
        return Vec::new();
    };
    db.list_files()
        .map(|entries| {
            entries
                .into_iter()
                .map(super::CompactBrowserEntry::from_wav_entry)
                .collect()
        })
        .unwrap_or_default()
}

fn compact_entries_from_loaded_pages(
    controller: &AppController,
) -> Option<Vec<super::CompactBrowserEntry>> {
    let total = controller.wav_entries.total;
    if total == 0 {
        return Some(Vec::new());
    }
    let loaded = controller
        .wav_entries
        .pages
        .values()
        .map(std::vec::Vec::len)
        .sum::<usize>();
    if loaded != total {
        return None;
    }

    let mut page_indices = controller
        .wav_entries
        .pages
        .keys()
        .copied()
        .collect::<Vec<_>>();
    page_indices.sort_unstable();
    let mut compact_entries = Vec::with_capacity(total);
    for page_index in page_indices {
        let page = controller.wav_entries.pages.get(&page_index)?;
        compact_entries.extend(
            page.iter()
                .cloned()
                .map(super::CompactBrowserEntry::from_wav_entry),
        );
    }
    Some(compact_entries)
}
