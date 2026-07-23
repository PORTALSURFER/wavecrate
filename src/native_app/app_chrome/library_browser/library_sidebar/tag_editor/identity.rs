use crate::native_app::ui::ids as widget_ids;

pub(super) const TAG_ENTRY_FIELD_KEY: &str = "metadata-tag-entry-field";
pub(super) const TAG_LIBRARY_TOGGLE_KEY: &str = "metadata-tag-library-toggle";

pub(super) fn metadata_tag_input_id() -> u64 {
    widget_ids::METADATA_TAG_INPUT_ID
}

pub(super) fn metadata_resize_header_id() -> u64 {
    widget_ids::METADATA_RESIZE_HEADER_ID
}

#[cfg(test)]
pub(super) fn metadata_sidebar_panel_id() -> u64 {
    widget_ids::METADATA_SIDEBAR_PANEL_ID
}

#[cfg(test)]
pub(super) fn metadata_tag_library_toggle_id() -> u64 {
    widget_ids::METADATA_TAG_LIBRARY_TOGGLE_ID
}

pub(super) fn tag_row_key(row_index: usize) -> String {
    format!("metadata-tag-row-{row_index}")
}

pub(super) fn accepted_tag_key(tag: &str) -> String {
    format!("metadata-tag-accepted-{tag}")
}

pub(super) fn accepted_tag_remove_key(tag: &str) -> String {
    format!("metadata-tag-remove-{tag}")
}

pub(super) fn pending_category_tag_key(tag: &str) -> String {
    format!("metadata-tag-pending-category-{tag}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_entry_row_keys_are_stable_by_projection_index() {
        assert_eq!(tag_row_key(0), "metadata-tag-row-0");
        assert_eq!(tag_row_key(12), "metadata-tag-row-12");
    }

    #[test]
    fn tag_token_keys_preserve_product_identity_text() {
        assert_eq!(
            accepted_tag_key("deep-kick"),
            "metadata-tag-accepted-deep-kick"
        );
        assert_eq!(
            accepted_tag_remove_key("deep-kick"),
            "metadata-tag-remove-deep-kick"
        );
        assert_eq!(
            pending_category_tag_key("deep-kick ->"),
            "metadata-tag-pending-category-deep-kick ->"
        );
    }

    #[test]
    fn metadata_sidebar_ids_remain_distinct() {
        let ids = [
            metadata_tag_input_id(),
            metadata_resize_header_id(),
            metadata_sidebar_panel_id(),
            metadata_tag_library_toggle_id(),
        ];
        for (index, id) in ids.iter().enumerate() {
            assert!(!ids[..index].contains(id));
        }
    }
}
