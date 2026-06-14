use super::*;
use std::path::Path;

pub(super) fn remap_path_scoped_browser_caches(
    controller: &mut AppController,
    source_id: &SourceId,
    old_path: &Path,
    new_path: &Path,
) {
    if old_path == new_path {
        return;
    }
    controller
        .runtime
        .source_lane
        .mutations
        .remap_looped_metadata_intent(source_id, old_path, new_path);
    if let Some(cache) = controller.ui_cache.browser.bpm_values.get_mut(source_id)
        && let Some(value) = cache.remove(old_path)
    {
        cache.insert(new_path.to_path_buf(), value);
    }
    if let Some(cache) = controller.ui_cache.browser.durations.get_mut(source_id)
        && let Some(value) = cache.remove(old_path)
    {
        cache.insert(new_path.to_path_buf(), value);
    }
    if let Some(cache) = controller
        .ui_cache
        .browser
        .analysis_failures
        .get_mut(source_id)
        && let Some(value) = cache.remove(old_path)
    {
        cache.insert(new_path.to_path_buf(), value);
    }
    if let Some(cache) = controller.ui_cache.browser.normal_tags.get_mut(source_id)
        && let Some(value) = cache.remove(old_path)
    {
        cache.insert(new_path.to_path_buf(), value);
    }
}
