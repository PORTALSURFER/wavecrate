use super::GuiAppState;
use super::GuiMessage;
use radiant::prelude as ui;
use radiant::widgets::TextInputMessage;
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    time::SystemTime,
};
use wavecrate::sample_sources::{SampleSource, SourceDatabase, SourceDbError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagCommit {
    pub(super) tags: Vec<String>,
    pub(super) remainder: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MetadataTagPersistResult {
    tags: Vec<String>,
    result: Result<(), String>,
}

#[derive(Clone, Debug)]
struct MetadataTagPersistRequest {
    absolute_path: PathBuf,
    source_root: PathBuf,
    relative_path: PathBuf,
    tags: Vec<String>,
}

impl GuiAppState {
    pub(super) fn load_persisted_metadata_tags(
        sources: &[SampleSource],
    ) -> Result<HashMap<String, Vec<String>>, String> {
        let mut tags_by_file = HashMap::new();
        let mut errors = Vec::new();
        for source in sources {
            if let Err(error) =
                load_persisted_metadata_tags_for_source(&source.root, &mut tags_by_file)
            {
                errors.push(format!("{}: {error}", source.root.display()));
            }
        }
        if errors.is_empty() {
            Ok(tags_by_file)
        } else {
            Err(errors.join("; "))
        }
    }

    pub(super) fn refresh_persisted_metadata_tags_for_source(&mut self, source_id: &str) {
        let Some(root) = self.folder_browser.source_root_path(source_id) else {
            return;
        };
        if let Err(error) =
            load_persisted_metadata_tags_for_source(&root, &mut self.metadata_tags_by_file)
        {
            self.sample_status = format!("Tags not loaded: {error}");
        }
    }

    pub(super) fn selected_metadata_tags(&self) -> &[String] {
        self.folder_browser
            .selected_file_id()
            .and_then(|file_id| self.metadata_tags_by_file.get(file_id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(super) fn apply_metadata_tag_input(
        &mut self,
        message: TextInputMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            TextInputMessage::Changed { value } => {
                self.metadata_tag_draft = value;
                self.reset_metadata_tag_completion_cycle();
            }
            TextInputMessage::Submitted { value } => {
                let mut commit = commit_metadata_tag_text(&value);
                let mut tags = std::mem::take(&mut self.metadata_tag_tokens);
                if tags.is_empty()
                    && commit.tags.len() <= 1
                    && let Some(tag) = self.metadata_tag_suggestion()
                {
                    tags.push(tag);
                    commit.tags.clear();
                }
                tags.append(&mut commit.tags);
                self.metadata_tag_draft.clear();
                self.reset_metadata_tag_completion_cycle();
                self.add_metadata_tags(tags, context);
            }
            TextInputMessage::CompletionRequested { value } => {
                self.metadata_tag_draft = value;
                let suggestions = self.metadata_tag_suggestions();
                match suggestions.as_slice() {
                    [] => self.reset_metadata_tag_completion_cycle(),
                    [tag] => self.stage_metadata_tag_token(tag.clone()),
                    _ => {
                        self.advance_metadata_tag_completion_cycle();
                    }
                }
            }
        }
    }

    pub(super) fn metadata_tag_suggestion(&self) -> Option<String> {
        let prefix = normalize_metadata_tag(&self.metadata_tag_draft)?;
        let suggestions = metadata_tag_completions_for_prefix(
            prefix.as_str(),
            self.known_metadata_tags().iter().map(String::as_str),
        );
        if suggestions.is_empty() {
            return None;
        }
        let index = if self.metadata_tag_completion_prefix.as_deref() == Some(prefix.as_str()) {
            self.metadata_tag_completion_index
        } else {
            0
        };
        suggestions.get(index % suggestions.len()).cloned()
    }

    fn metadata_tag_suggestions(&self) -> Vec<String> {
        let Some(prefix) = normalize_metadata_tag(&self.metadata_tag_draft) else {
            return Vec::new();
        };
        metadata_tag_completions_for_prefix(
            prefix.as_str(),
            self.known_metadata_tags().iter().map(String::as_str),
        )
    }

    pub(super) fn known_metadata_tags(&self) -> Vec<String> {
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
        self.reset_metadata_tag_completion_cycle();
    }

    fn advance_metadata_tag_completion_cycle(&mut self) {
        let Some(prefix) = normalize_metadata_tag(&self.metadata_tag_draft) else {
            self.reset_metadata_tag_completion_cycle();
            return;
        };
        let suggestions = metadata_tag_completions_for_prefix(
            prefix.as_str(),
            self.known_metadata_tags().iter().map(String::as_str),
        );
        if suggestions.is_empty() {
            self.reset_metadata_tag_completion_cycle();
            return;
        }
        if self.metadata_tag_completion_prefix.as_deref() == Some(prefix.as_str()) {
            self.metadata_tag_completion_index =
                (self.metadata_tag_completion_index + 1) % suggestions.len();
        } else {
            self.metadata_tag_completion_prefix = Some(prefix);
            self.metadata_tag_completion_index = 1 % suggestions.len();
        }
    }

    fn reset_metadata_tag_completion_cycle(&mut self) {
        self.metadata_tag_completion_prefix = None;
        self.metadata_tag_completion_index = 0;
    }

    pub(super) fn add_metadata_tags(
        &mut self,
        tags: Vec<String>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(file_id) = self.folder_browser.selected_file_id().map(str::to_owned) else {
            self.sample_status = String::from("Select a sample before adding tags");
            return;
        };
        let absolute_path = PathBuf::from(&file_id);
        let Some((source_root, relative_path)) = self
            .folder_browser
            .source_relative_file_path(&absolute_path)
        else {
            self.sample_status = String::from("Selected sample is not inside a configured source");
            return;
        };
        let mut file_tags = self
            .metadata_tags_by_file
            .get(&file_id)
            .cloned()
            .unwrap_or_default();
        let mut added = Vec::new();
        for tag in tags {
            if file_tags.iter().any(|existing| existing == &tag) {
                continue;
            }
            file_tags.push(tag.clone());
            added.push(tag);
        }
        self.metadata_tags_by_file
            .insert(file_id.clone(), file_tags);
        match added.as_slice() {
            [] => {}
            [tag] => self.sample_status = format!("Added tag {tag}"),
            tags => self.sample_status = format!("Added {} tags", tags.len()),
        }
        if !added.is_empty() {
            let request = MetadataTagPersistRequest {
                absolute_path,
                source_root,
                relative_path,
                tags: added,
            };
            context.spawn(
                "gui-metadata-tag-persist",
                move || persist_metadata_tag_additions(request),
                GuiMessage::MetadataTagsPersisted,
            );
        }
    }

    pub(super) fn finish_metadata_tag_persist(&mut self, result: MetadataTagPersistResult) {
        if let Err(error) = result.result {
            self.sample_status = match result.tags.as_slice() {
                [tag] => format!("Tag {tag} not saved: {error}"),
                tags => format!("{} tags not saved: {error}", tags.len()),
            };
        }
    }
}

fn persist_metadata_tag_additions(request: MetadataTagPersistRequest) -> MetadataTagPersistResult {
    let result = persist_metadata_tag_additions_inner(&request);
    MetadataTagPersistResult {
        tags: request.tags,
        result,
    }
}

fn persist_metadata_tag_additions_inner(request: &MetadataTagPersistRequest) -> Result<(), String> {
    let (file_size, modified_ns) = file_metadata(&request.absolute_path)?;
    let db = SourceDatabase::open_for_user_metadata_write(&request.source_root)
        .map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    batch
        .upsert_file(&request.relative_path, file_size, modified_ns)
        .map_err(|err| err.to_string())?;
    for tag in &request.tags {
        batch
            .assign_tag_to_path(&request.relative_path, tag)
            .map_err(|err| err.to_string())?;
    }
    batch.commit().map_err(|err| err.to_string())
}

#[cfg(test)]
pub(super) fn persist_metadata_tag_additions_for_tests(
    absolute_path: PathBuf,
    source_root: PathBuf,
    relative_path: PathBuf,
    tags: Vec<String>,
) -> Result<(), String> {
    persist_metadata_tag_additions_inner(&MetadataTagPersistRequest {
        absolute_path,
        source_root,
        relative_path,
        tags,
    })
}

fn load_persisted_metadata_tags_for_source(
    source_root: &Path,
    tags_by_file: &mut HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let db = match SourceDatabase::open_read_only(source_root) {
        Ok(db) => db,
        Err(SourceDbError::ReadOnlyDatabaseMissing(_)) => return Ok(()),
        Err(err) => return Err(err.to_string()),
    };
    for entry in db.list_files().map_err(|err| err.to_string())? {
        if entry.normal_tags.is_empty() {
            continue;
        }
        let absolute_path = source_root.join(entry.relative_path);
        tags_by_file.insert(
            absolute_path.to_string_lossy().to_string(),
            entry.normal_tags,
        );
    }
    Ok(())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Sample metadata unavailable for {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_nanos()).ok())
        .unwrap_or_default();
    Ok((metadata.len(), modified_ns))
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

#[cfg(test)]
pub(super) fn metadata_tag_completion<'a>(
    value: &str,
    known_tags: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let prefix = normalize_metadata_tag(value)?;
    metadata_tag_completions_for_prefix(prefix.as_str(), known_tags)
        .into_iter()
        .next()
}

fn metadata_tag_completions_for_prefix<'a>(
    prefix: &str,
    known_tags: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    known_tags
        .into_iter()
        .filter(|tag| tag.starts_with(prefix))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
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
