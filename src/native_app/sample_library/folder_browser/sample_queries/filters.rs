use std::collections::HashMap;

use super::FileEntry;

pub(super) fn filter_audio_files_by_name(files: &mut Vec<&FileEntry>, name_filter: &str) {
    let query = normalized_name_filter(name_filter);
    files.retain(|file| audio_file_matches_name_query(file, &query));
}

pub(super) fn filter_audio_files_by_tags(
    files: &mut Vec<&FileEntry>,
    tags_by_file: &HashMap<String, Vec<String>>,
    tag_filter: &str,
) {
    let required_tags = parsed_tag_filter(tag_filter);
    files.retain(|file| audio_file_matches_parsed_tags(file, tags_by_file, &required_tags));
}

pub(super) fn audio_file_matches_name_query(file: &FileEntry, query: &str) -> bool {
    query.is_empty()
        || file.name.to_ascii_lowercase().contains(query)
        || file.stem.to_ascii_lowercase().contains(query)
}

pub(super) fn audio_file_matches_parsed_tags(
    file: &FileEntry,
    tags_by_file: &HashMap<String, Vec<String>>,
    required_tags: &[String],
) -> bool {
    if required_tags.is_empty() {
        return true;
    }
    let Some(file_tags) = tags_by_file.get(&file.id) else {
        return false;
    };
    required_tags.iter().all(|required| {
        file_tags
            .iter()
            .any(|tag| tag.trim().eq_ignore_ascii_case(required))
    })
}

pub(super) fn parsed_tag_filter(tag_filter: &str) -> Vec<String> {
    tag_filter
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_ascii_lowercase())
        .collect()
}

pub(super) fn normalized_name_filter(name_filter: &str) -> String {
    name_filter.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{normalized_name_filter, parsed_tag_filter};

    #[test]
    fn parses_tag_filter_as_trimmed_case_insensitive_requirements() {
        assert_eq!(
            parsed_tag_filter(" Drum, warm ,, Lo-Fi "),
            vec!["drum", "warm", "lo-fi"]
        );
    }

    #[test]
    fn normalizes_name_filter_for_case_insensitive_matching() {
        assert_eq!(normalized_name_filter("  Deep Kick  "), "deep kick");
    }
}
