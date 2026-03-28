use super::super::super::projection_key_encoding::{
    encode_focus_context, encode_update_status, normalized_f32_to_milli,
};
use super::super::NonSegmentStaticProjectionCacheKey;
use super::shared::{hash_path_for_projection_key, hash_string_for_projection_key};
use crate::app_core::controller::AppController;
use crate::app_core::state::InlineFolderEditKind;

/// Build a projection key for static fields outside explicit segment keys.
pub(super) fn build_non_segment_static_projection_key(
    controller: &AppController,
) -> NonSegmentStaticProjectionCacheKey {
    let folder_inline = controller.ui.sources.folders.inline_edit.as_ref();
    NonSegmentStaticProjectionCacheKey {
        sources_selected: controller.ui.sources.selected,
        sources_len: controller.ui.sources.rows.len(),
        folder_rows_len: controller.ui.sources.folders.rows.len(),
        folder_focused: controller.ui.sources.folders.focused,
        folder_search_revision: controller.ui.projection_revisions.folder_search,
        folder_inline_kind: match folder_inline.map(|draft| &draft.kind) {
            None => 0,
            Some(InlineFolderEditKind::Create { .. }) => 1,
            Some(InlineFolderEditKind::Rename { .. }) => 2,
        },
        folder_inline_path_hash: folder_inline.map(|draft| match &draft.kind {
            InlineFolderEditKind::Create { parent } => {
                hash_path_for_projection_key(parent.as_path())
            }
            InlineFolderEditKind::Rename { target } => {
                hash_path_for_projection_key(target.as_path())
            }
        }),
        folder_inline_name_hash: folder_inline
            .map(|draft| hash_string_for_projection_key(draft.name.as_str())),
        folder_inline_focus_requested: folder_inline.is_some_and(|draft| draft.focus_requested),
        folder_inline_select_all_on_focus: folder_inline
            .is_some_and(|draft| draft.select_all_on_focus_requested),
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
