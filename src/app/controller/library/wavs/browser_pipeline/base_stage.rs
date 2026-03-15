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

    let mut base_rows = Vec::with_capacity(controller.wav_entries_len());
    let mut trash_rows = Vec::new();
    let mut neutral_rows = Vec::new();
    let mut keep_rows = Vec::new();
    let _ = controller.for_each_wav_entry(|index, entry| {
        base_rows.push(index);
        if entry.tag.is_trash() {
            trash_rows.push(index);
        } else if entry.tag.is_keep() {
            keep_rows.push(index);
        } else {
            neutral_rows.push(index);
        }
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
