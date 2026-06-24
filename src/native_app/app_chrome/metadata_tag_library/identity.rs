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

pub(super) fn category_disclosure_key(category_id: &str) -> String {
    format!("metadata-tag-category-disclosure-{category_id}")
}

pub(super) fn category_label_key(category_id: &str) -> String {
    format!("metadata-tag-category-label-{category_id}")
}

pub(super) fn category_underlay_key(category_id: &str) -> String {
    format!("metadata-tag-category-{category_id}")
}

pub(super) fn tag_row_key(tag: &str) -> String {
    format!("metadata-tag-library-row-{tag}")
}

pub(super) fn empty_category_key(category_id: &str) -> String {
    format!("metadata-tag-empty-category-{category_id}")
}
