use super::persistence::{persist_metadata_tag_assignment, persist_metadata_tag_assignments};
use super::types::{MetadataTagPersistRequest, MetadataTagPersistResult};
use super::{GuiMessage, MetadataMessage, NativeAppState};
use crate::native_app::audio::playback::tagged_playback_mode_for_tag;
use radiant::prelude as ui;
use std::path::PathBuf;

struct MetadataTagTarget {
    file_id: String,
    absolute_path: PathBuf,
    source_root: PathBuf,
    relative_path: PathBuf,
}

impl NativeAppState {
    pub(in crate::native_app) fn add_metadata_tags(
        &mut self,
        tags: Vec<String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let targets = match self.selected_metadata_tag_targets("adding") {
            Ok(targets) => targets,
            Err(status) => {
                self.ui.status.sample = status;
                return;
            }
        };
        let mut requests = Vec::new();
        let mut changed_files = Vec::new();
        for target in targets {
            let mut file_tags = self
                .metadata
                .tags_by_file
                .get(&target.file_id)
                .cloned()
                .unwrap_or_default();
            let mut added = Vec::new();
            let mut removed_conflicting = Vec::new();
            for tag in &tags {
                let removed = remove_conflicting_playback_tags(&mut file_tags, tag, &mut added);
                extend_unique(&mut removed_conflicting, removed);
                if file_tags.iter().any(|existing| existing == tag) {
                    continue;
                }
                file_tags.push(tag.clone());
                push_unique(&mut added, tag.clone());
            }
            if added.is_empty() && removed_conflicting.is_empty() {
                continue;
            }
            self.metadata
                .tags_by_file
                .insert(target.file_id.clone(), file_tags);
            if self
                .metadata
                .selected_tag
                .as_ref()
                .is_some_and(|selected| removed_conflicting.iter().any(|tag| tag == selected))
            {
                self.metadata.selected_tag = None;
            }
            self.reconcile_playback_mode_after_metadata_tag_change(target.file_id.as_str());
            changed_files.push(target.file_id.clone());
            if !removed_conflicting.is_empty() {
                requests.push(MetadataTagPersistRequest {
                    absolute_path: target.absolute_path.clone(),
                    source_root: target.source_root.clone(),
                    relative_path: target.relative_path.clone(),
                    tags: removed_conflicting,
                    assigned: false,
                });
            }
            if !added.is_empty() {
                requests.push(MetadataTagPersistRequest {
                    absolute_path: target.absolute_path,
                    source_root: target.source_root,
                    relative_path: target.relative_path,
                    tags: added,
                    assigned: true,
                });
            }
        }
        if requests.is_empty() {
            return;
        }
        self.retain_visible_file_selection_after_metadata_tag_change();
        let status_tags = added_metadata_tag_status_tags(&requests, &tags);
        self.ui.status.sample = metadata_tag_added_status(&status_tags, changed_files.len());
        if requests.len() == 1 {
            let request = requests.remove(0);
            context
                .business()
                .background("gui-metadata-tag-persist")
                .run(
                    move |_| persist_metadata_tag_assignment(request),
                    |result| GuiMessage::Metadata(MetadataMessage::MetadataTagsPersisted(result)),
                );
        } else {
            context
                .business()
                .background("gui-metadata-tag-persist")
                .run(
                    move |_| persist_metadata_tag_assignments(requests),
                    |result| GuiMessage::Metadata(MetadataMessage::MetadataTagsPersisted(result)),
                );
        }
    }

    pub(in crate::native_app) fn toggle_metadata_tag(
        &mut self,
        tag: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.metadata_tag_selection_state(&tag).is_all() {
            self.remove_metadata_tag(tag, context);
        } else {
            self.add_metadata_tags(vec![tag], context);
        }
    }

    pub(in crate::native_app) fn remove_selected_metadata_tag(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(tag) = self.metadata.selected_tag.clone() else {
            return;
        };
        self.remove_metadata_tag(tag, context);
    }

    fn remove_metadata_tag(&mut self, tag: String, context: &mut ui::UiUpdateContext<GuiMessage>) {
        let targets = match self.selected_metadata_tag_targets("removing") {
            Ok(targets) => targets,
            Err(status) => {
                self.ui.status.sample = status;
                return;
            }
        };
        let mut requests = Vec::new();
        let mut changed_files = Vec::new();
        for target in targets {
            let Some(file_tags) = self.metadata.tags_by_file.get_mut(&target.file_id) else {
                continue;
            };
            let before_len = file_tags.len();
            file_tags.retain(|existing| existing != &tag);
            if file_tags.len() == before_len {
                continue;
            }
            if file_tags.is_empty() {
                self.metadata.tags_by_file.remove(&target.file_id);
            }
            self.reconcile_playback_mode_after_metadata_tag_change(target.file_id.as_str());
            changed_files.push(target.file_id.clone());
            requests.push(MetadataTagPersistRequest {
                absolute_path: target.absolute_path,
                source_root: target.source_root,
                relative_path: target.relative_path,
                tags: vec![tag.clone()],
                assigned: false,
            });
        }
        if requests.is_empty() {
            return;
        }
        if self.metadata.selected_tag.as_deref() == Some(tag.as_str()) {
            self.metadata.selected_tag = None;
        }
        self.retain_visible_file_selection_after_metadata_tag_change();
        self.ui.status.sample = metadata_tag_removed_status(&tag, changed_files.len());
        if requests.len() == 1 {
            let request = requests.remove(0);
            context
                .business()
                .background("gui-metadata-tag-persist")
                .run(
                    move |_| persist_metadata_tag_assignment(request),
                    |result| GuiMessage::Metadata(MetadataMessage::MetadataTagsPersisted(result)),
                );
        } else {
            context
                .business()
                .background("gui-metadata-tag-persist")
                .run(
                    move |_| super::persistence::persist_metadata_tag_deletions(requests),
                    |result| GuiMessage::Metadata(MetadataMessage::MetadataTagsPersisted(result)),
                );
        }
    }

    pub(in crate::native_app) fn finish_metadata_tag_persist(
        &mut self,
        result: MetadataTagPersistResult,
    ) {
        if let Err(error) = result.result {
            self.ui.status.sample = match result.tags.as_slice() {
                [tag] if result.assigned => format!("Tag {tag} not saved: {error}"),
                [tag] => format!("Tag {tag} not removed: {error}"),
                tags if result.assigned => format!("{} tags not saved: {error}", tags.len()),
                tags => format!("{} tags not removed: {error}", tags.len()),
            };
        }
    }

    fn selected_metadata_tag_targets(
        &self,
        action: &'static str,
    ) -> Result<Vec<MetadataTagTarget>, String> {
        let paths = self.library.folder_browser.selected_file_paths();
        if paths.is_empty() {
            return Err(format!("Select a sample before {action} tags"));
        }
        paths
            .into_iter()
            .map(|absolute_path| {
                let Some((source_root, relative_path)) = self
                    .library
                    .folder_browser
                    .source_relative_file_path(&absolute_path)
                else {
                    return Err(String::from(
                        "Selected sample is not inside a configured source",
                    ));
                };
                Ok(MetadataTagTarget {
                    file_id: absolute_path.to_string_lossy().to_string(),
                    absolute_path,
                    source_root,
                    relative_path,
                })
            })
            .collect()
    }
}

fn remove_conflicting_playback_tags(
    file_tags: &mut Vec<String>,
    incoming: &str,
    added: &mut Vec<String>,
) -> Vec<String> {
    let Some(incoming_mode) = tagged_playback_mode_for_tag(incoming) else {
        return Vec::new();
    };
    let mut removed = Vec::new();
    file_tags.retain(|existing| {
        let conflicts = tagged_playback_mode_for_tag(existing)
            .is_some_and(|existing_mode| existing_mode != incoming_mode);
        if conflicts && !added.iter().any(|added_tag| added_tag == existing) {
            push_unique(&mut removed, existing.clone());
        }
        !conflicts
    });
    added.retain(|existing| {
        !tagged_playback_mode_for_tag(existing)
            .is_some_and(|existing_mode| existing_mode != incoming_mode)
    });
    removed
}

fn push_unique(tags: &mut Vec<String>, tag: String) {
    if !tags.iter().any(|existing| existing == &tag) {
        tags.push(tag);
    }
}

fn extend_unique(tags: &mut Vec<String>, incoming: Vec<String>) {
    for tag in incoming {
        push_unique(tags, tag);
    }
}

fn added_metadata_tag_status_tags(
    requests: &[MetadataTagPersistRequest],
    requested_tags: &[String],
) -> Vec<String> {
    let mut tags = Vec::new();
    for request in requests.iter().filter(|request| request.assigned) {
        extend_unique(&mut tags, request.tags.clone());
    }
    if tags.is_empty() {
        extend_unique(&mut tags, requested_tags.to_vec());
    }
    tags
}

fn metadata_tag_added_status(tags: &[String], changed_file_count: usize) -> String {
    match (tags, changed_file_count) {
        ([tag], 1) => format!("Added tag {tag}"),
        ([tag], count) => format!("Added tag {tag} to {count} samples"),
        (tags, 1) => format!("Added {} tags", tags.len()),
        (tags, count) => format!("Added {} tags to {count} samples", tags.len()),
    }
}

fn metadata_tag_removed_status(tag: &str, changed_file_count: usize) -> String {
    if changed_file_count == 1 {
        format!("Removed tag {tag}")
    } else {
        format!("Removed tag {tag} from {changed_file_count} samples")
    }
}
