use super::super::SourceDbError;

pub(in crate::sample_sources::db) struct NormalizedTagIdentity {
    pub(in crate::sample_sources::db) display_label: String,
    pub(in crate::sample_sources::db) normalized_text: String,
}

pub(in crate::sample_sources::db) fn normalize_tag_identity(
    label: &str,
) -> Result<NormalizedTagIdentity, SourceDbError> {
    let display_label = label.split_whitespace().collect::<Vec<_>>().join(" ");
    if display_label.is_empty() {
        return Err(SourceDbError::EmptyTagLabel);
    }

    Ok(NormalizedTagIdentity {
        normalized_text: display_label.to_ascii_lowercase(),
        display_label,
    })
}
