use super::super::super::projection_key_encoding::{
    encode_focus_context, encode_update_status, normalized_f32_to_milli,
};
use super::super::NonSegmentStaticProjectionCacheKey;
use super::shared::{hash_path_for_projection_key, hash_string_for_projection_key};
use crate::app_core::controller::AppController;

/// Build a projection key for static fields outside explicit segment keys.
pub(super) fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> NonSegmentStaticProjectionCacheKey {
    let folder_create = controller.ui.sources.folders.new_folder.as_ref();
    NonSegmentStaticProjectionCacheKey {
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        folder_create_parent_hash: folder_create
            .map(|draft| hash_path_for_projection_key(draft.parent.as_path())),
        folder_create_name_hash: folder_create
            .map(|draft| hash_string_for_projection_key(draft.name.as_str())),
        folder_create_focus_requested: folder_create.is_some_and(|draft| draft.focus_requested),
        update_status: encode_update_status(&controller.ui.update.status),
        update_revision: controller.ui.projection_revisions.update,
        volume_milli: normalized_f32_to_milli(controller.ui.volume),
        transport_running: controller.is_playing(),
        focus_context: encode_focus_context(controller.ui.focus.context),
        trash_count: controller.ui.browser.trash.len(),
        neutral_count: controller.ui.browser.neutral.len(),
        keep_count: controller.ui.browser.keep.len(),
    }
}
