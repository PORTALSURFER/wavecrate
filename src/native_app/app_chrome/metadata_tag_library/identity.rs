pub(super) const PANEL_KEY: &str = "metadata-tag-library-panel";

pub(super) fn category_drop_indicator_key(category_id: &str) -> String {
    format!("metadata-tag-category-drop-indicator-{category_id}")
}

pub(super) fn category_pills_key(category_id: &str) -> String {
    format!("metadata-tag-category-pills-{category_id}")
}

pub(super) fn category_group_key(category_id: &str) -> String {
    format!("metadata-tag-category-group-{category_id}")
}

pub(super) fn category_input_key(category_id: &str) -> String {
    format!("metadata-tag-category-{category_id}")
}

pub(super) fn tag_row_key(tag: &str) -> String {
    format!("metadata-tag-library-row-{tag}")
}

pub(super) fn empty_category_input_key(category_id: &str) -> String {
    format!("metadata-tag-empty-category-{category_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_input_keys_keep_header_and_empty_drop_target_distinct() {
        assert_eq!(
            category_input_key("character"),
            "metadata-tag-category-character"
        );
        assert_eq!(
            empty_category_input_key("character"),
            "metadata-tag-empty-category-character"
        );
        assert_ne!(
            category_input_key("character"),
            empty_category_input_key("character")
        );
    }
}
