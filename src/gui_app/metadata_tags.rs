use super::GuiAppState;
use radiant::widgets::TextInputMessage;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagCommit {
    pub(super) tags: Vec<String>,
    pub(super) remainder: String,
}

impl GuiAppState {
    pub(super) fn apply_metadata_tag_input(&mut self, message: TextInputMessage) {
        match message {
            TextInputMessage::Changed { value } => {
                let commit = commit_metadata_tag_text(&value, false);
                self.metadata_tag_draft = commit.remainder;
                self.add_metadata_tags(commit.tags);
            }
            TextInputMessage::Submitted { value } => {
                let commit = commit_metadata_tag_text(&value, true);
                self.metadata_tag_draft.clear();
                self.add_metadata_tags(commit.tags);
            }
        }
    }

    fn add_metadata_tags(&mut self, tags: Vec<String>) {
        let mut added = Vec::new();
        for tag in tags {
            if self.metadata_tags.iter().any(|existing| existing == &tag) {
                continue;
            }
            self.metadata_tags.push(tag.clone());
            added.push(tag);
        }
        match added.as_slice() {
            [] => {}
            [tag] => self.sample_status = format!("Added tag {tag}"),
            tags => self.sample_status = format!("Added {} tags", tags.len()),
        }
    }
}

pub(super) fn commit_metadata_tag_text(value: &str, submit: bool) -> MetadataTagCommit {
    let mut parts = value
        .split(['\n', ',', ';'])
        .map(str::trim)
        .collect::<Vec<_>>();
    let remainder = if submit {
        ""
    } else {
        parts.pop().unwrap_or_default()
    };
    MetadataTagCommit {
        tags: parts
            .into_iter()
            .filter_map(normalize_metadata_tag)
            .collect(),
        remainder: remainder.to_string(),
    }
}

pub(super) fn normalize_metadata_tag(value: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut previous_separator = false;
    for ch in value.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() {
            previous_separator = false;
            ch.to_ascii_lowercase()
        } else if ch == '_' {
            previous_separator = false;
            ch
        } else if ch.is_whitespace() || ch == '-' || ch.is_ascii_punctuation() {
            if previous_separator || normalized.is_empty() {
                previous_separator = true;
                continue;
            }
            previous_separator = true;
            '-'
        } else {
            continue;
        };
        normalized.push(next);
    }
    let normalized = normalized.trim_matches('-').to_string();
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_tags_normalize_to_single_token_values() {
        assert_eq!(
            normalize_metadata_tag("Deep Kick 01"),
            Some(String::from("deep-kick-01"))
        );
        assert_eq!(
            normalize_metadata_tag("  metal_floor  "),
            Some(String::from("metal_floor"))
        );
        assert_eq!(normalize_metadata_tag("!!!"), None);
    }

    #[test]
    fn changed_tag_input_commits_completed_delimited_tags_only() {
        assert_eq!(
            commit_metadata_tag_text("kick, warm tone", false),
            MetadataTagCommit {
                tags: vec![String::from("kick")],
                remainder: String::from("warm tone"),
            }
        );
    }

    #[test]
    fn submitted_tag_input_commits_the_full_field() {
        assert_eq!(
            commit_metadata_tag_text("warm tone", true),
            MetadataTagCommit {
                tags: vec![String::from("warm-tone")],
                remainder: String::new(),
            }
        );
    }
}
