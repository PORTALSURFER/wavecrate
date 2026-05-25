use super::GuiAppState;
use radiant::widgets::TextInputMessage;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagCommit {
    pub(super) tags: Vec<String>,
    pub(super) remainder: String,
}

impl GuiAppState {
    pub(super) fn selected_metadata_tags(&self) -> &[String] {
        self.folder_browser
            .selected_file_id()
            .and_then(|file_id| self.metadata_tags_by_file.get(file_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(super) fn apply_metadata_tag_input(&mut self, message: TextInputMessage) {
        match message {
            TextInputMessage::Changed { value } => {
                self.metadata_tag_draft = value;
            }
            TextInputMessage::Submitted { value } => {
                let mut commit = commit_metadata_tag_text(&value);
                let mut tags = std::mem::take(&mut self.metadata_tag_tokens);
                tags.append(&mut commit.tags);
                self.metadata_tag_draft.clear();
                self.add_metadata_tags(tags);
            }
            TextInputMessage::CompletionRequested { value } => {
                self.metadata_tag_draft = value;
                if let Some(tag) = self.metadata_tag_suggestion() {
                    self.stage_metadata_tag_token(tag);
                }
            }
        }
    }

    pub(super) fn metadata_tag_suggestion(&self) -> Option<String> {
        metadata_tag_completion(
            &self.metadata_tag_draft,
            self.known_metadata_tags().iter().map(String::as_str),
        )
    }

    fn known_metadata_tags(&self) -> Vec<String> {
        self.metadata_tags_by_file
            .values()
            .flat_map(|tags| tags.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn stage_metadata_tag_token(&mut self, tag: String) {
        if !self
            .metadata_tag_tokens
            .iter()
            .any(|existing| existing == &tag)
        {
            self.metadata_tag_tokens.push(tag);
        }
        self.metadata_tag_draft.clear();
    }

    fn add_metadata_tags(&mut self, tags: Vec<String>) {
        let Some(file_id) = self.folder_browser.selected_file_id().map(str::to_owned) else {
            self.sample_status = String::from("Select a sample before adding tags");
            return;
        };
        let file_tags = self.metadata_tags_by_file.entry(file_id).or_default();
        let mut added = Vec::new();
        for tag in tags {
            if file_tags.iter().any(|existing| existing == &tag) {
                continue;
            }
            file_tags.push(tag.clone());
            added.push(tag);
        }
        match added.as_slice() {
            [] => {}
            [tag] => self.sample_status = format!("Added tag {tag}"),
            tags => self.sample_status = format!("Added {} tags", tags.len()),
        }
    }
}

pub(super) fn commit_metadata_tag_text(value: &str) -> MetadataTagCommit {
    let parts = value
        .split(['\n', ',', ';'])
        .map(str::trim)
        .collect::<Vec<_>>();
    MetadataTagCommit {
        tags: parts
            .into_iter()
            .filter_map(normalize_metadata_tag)
            .collect(),
        remainder: String::new(),
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

pub(super) fn metadata_tag_completion<'a>(
    value: &str,
    known_tags: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let prefix = normalize_metadata_tag(value)?;
    known_tags
        .into_iter()
        .filter(|tag| tag.starts_with(prefix.as_str()))
        .min_by_key(|tag| (tag.len(), *tag))
        .map(str::to_string)
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
    fn submitted_tag_input_commits_delimited_tags() {
        assert_eq!(
            commit_metadata_tag_text("kick, warm tone"),
            MetadataTagCommit {
                tags: vec![String::from("kick"), String::from("warm-tone")],
                remainder: String::new(),
            }
        );
    }

    #[test]
    fn metadata_tag_completion_matches_known_tag_prefix() {
        assert_eq!(
            metadata_tag_completion("ki", ["warm", "kick", "kicker"].into_iter()),
            Some(String::from("kick"))
        );
        assert_eq!(metadata_tag_completion("zz", ["kick"].into_iter()), None);
    }
}
