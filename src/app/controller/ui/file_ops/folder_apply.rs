//! Folder mutation result application helpers.

use super::*;

impl AppController {
    pub(super) fn apply_folder_create_result(&mut self, result: FolderCreateResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.relative_path.clone()]);
        match result.result {
            Ok(()) => {
                self.update_manual_folders(|set| {
                    set.insert(result.relative_path.clone());
                });
                self.update_disk_folders(|set| {
                    set.insert(result.relative_path.clone());
                });
                self.refresh_folder_browser();
                self.focus_folder_by_path(&result.relative_path);
                self.set_status(
                    format!("Created folder {}", result.relative_path.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    pub(super) fn apply_folder_rename_result(&mut self, result: FolderRenameResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.old_folder.clone()]);
        match result.result {
            Ok(()) => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                else {
                    self.set_status("Source not available for folder rename", StatusTone::Error);
                    return;
                };
                for entry in result.entries {
                    let old_relative = result.old_folder.join(
                        entry
                            .relative_path
                            .strip_prefix(&result.new_folder)
                            .unwrap_or(entry.relative_path.as_path()),
                    );
                    self.update_cached_entry(&source, &old_relative, entry.clone());
                }
                self.remap_folder_state(&result.old_folder, &result.new_folder);
                self.remap_manual_folders(&result.old_folder, &result.new_folder);
                self.refresh_folder_browser();
                self.focus_folder_by_path(&result.new_folder);
                self.set_status(
                    format!("Renamed folder to {}", result.new_folder.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    pub(super) fn apply_folder_delete_result(&mut self, result: FolderDeleteResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.relative_path.clone()]);
        match result.result {
            Ok(()) => {
                let source = SampleSource {
                    id: result.source_id.clone(),
                    root: result.source_root.clone(),
                };
                self.apply_deleted_folder_state(
                    &source,
                    &result.relative_path,
                    result.next_focus.as_deref(),
                    &result.entries,
                );
                if let Some(staged) = result.staged {
                    let before = self.capture_meaningful_ui_snapshot();
                    let after = self.capture_meaningful_ui_snapshot();
                    let entry = self.deleted_folder_undo_entry(
                        source,
                        result.staging_root,
                        staged,
                        result.entries,
                        result.next_focus,
                    );
                    self.push_undo_entry(AppController::attach_meaningful_ui_restore(
                        entry, before, after,
                    ));
                }
                self.set_status(
                    format!("Deleted folder {}", result.relative_path.display()),
                    StatusTone::Info,
                );
            }
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }
}
