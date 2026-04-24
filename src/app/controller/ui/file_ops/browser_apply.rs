//! Browser sample result application helpers.

use super::*;

impl AppController {
    pub(super) fn apply_sample_delete_result(&mut self, result: SampleDeleteResult) {
        self.finish_pending_file_mutation(&result.source_id, result.requested_paths.clone());
        let selected_source_id = self.selected_source_id();
        let similar_query = self.ui.browser.search.similar_query.clone();
        for path in &result.deleted_paths {
            if let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == result.source_id)
                .cloned()
            {
                self.prune_cached_sample(&source, path);
            }
        }
        if !result.deleted_paths.is_empty() {
            crate::app::controller::library::wavs::schedule_similarity_filter_rebuild_after_delete_with_state(
                self,
                selected_source_id,
                similar_query,
                &result.deleted_paths.iter().cloned().collect::<std::collections::HashSet<_>>(),
            );
            crate::app::controller::library::wavs::apply_pending_similarity_filter_rebuild(self);
            self.browser()
                .restore_browser_focus_after_delete(result.next_focus);
            self.complete_file_op_status(
                format!("Deleted {} sample(s)", result.deleted_paths.len()),
                StatusTone::Info,
            );
        }
        if let Some(err) = result.last_error {
            self.complete_file_op_status(format!("Delete failed: {err}"), StatusTone::Error);
        }
    }

    pub(super) fn apply_sample_rename_result(&mut self, result: SampleRenameResult) {
        self.finish_pending_file_mutation(&result.source_id, [result.old_relative.clone()]);
        let mut queued_auto_rename = self
            .runtime
            .source_lane
            .mutations
            .finish_browser_rename_intent();
        match result.result {
            Ok(()) => {
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == result.source_id)
                    .cloned()
                else {
                    self.complete_file_op_status(
                        "Source not available for rename",
                        StatusTone::Error,
                    );
                    self.dispatch_queued_browser_auto_rename(queued_auto_rename);
                    return;
                };
                if let Some(entry) = result.entry {
                    self.update_cached_entry(&source, &result.old_relative, entry);
                }
                remap_queued_browser_auto_rename_path(
                    &mut queued_auto_rename,
                    &result.old_relative,
                    &result.new_relative,
                );
                if result.resume_playback {
                    self.runtime
                        .jobs
                        .set_pending_playback(Some(PendingPlayback {
                            source_id: result.source_id.clone(),
                            relative_path: result.new_relative.clone(),
                            looped: result.resume_looped,
                            start_override: result.resume_start_override,
                            force_loaded_audio: false,
                        }));
                }
                self.refresh_waveform_for_sample(&source, &result.new_relative);
                self.complete_file_op_status(
                    format!(
                        "Renamed {} to {}",
                        result.old_relative.display(),
                        result.new_relative.display()
                    ),
                    StatusTone::Info,
                );
            }
            Err(err) => self.complete_file_op_status(err, StatusTone::Error),
        }
        self.dispatch_queued_browser_auto_rename(queued_auto_rename);
    }

    pub(super) fn apply_sample_auto_rename_result(&mut self, result: SampleAutoRenameResult) {
        self.finish_pending_file_mutation(&result.source_id, result.requested_paths.clone());
        let mut queued_auto_rename = self
            .runtime
            .source_lane
            .mutations
            .finish_browser_rename_intent();
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == result.source_id)
            .cloned()
        else {
            self.complete_file_op_status("Source not available for auto rename", StatusTone::Error);
            self.dispatch_queued_browser_auto_rename(queued_auto_rename);
            return;
        };
        for renamed in &result.renamed {
            self.update_cached_entry(&source, &renamed.old_relative, renamed.entry.clone());
            remap_queued_browser_auto_rename_path(
                &mut queued_auto_rename,
                &renamed.old_relative,
                &renamed.new_relative,
            );
            if renamed.resume_playback {
                self.runtime
                    .jobs
                    .set_pending_playback(Some(PendingPlayback {
                        source_id: result.source_id.clone(),
                        relative_path: renamed.new_relative.clone(),
                        looped: renamed.resume_looped,
                        start_override: renamed.resume_start_override,
                        force_loaded_audio: false,
                    }));
            }
            self.refresh_waveform_for_sample(&source, &renamed.new_relative);
        }
        let renamed = result.renamed.len();
        let skipped = result.skipped.len();
        let failed = result.errors.len();
        let tone = if failed == 0 {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        self.complete_file_op_status(
            format!("Auto Rename: renamed {renamed}, skipped {skipped}, failed {failed}"),
            tone,
        );
        self.dispatch_queued_browser_auto_rename(queued_auto_rename);
    }

    fn dispatch_queued_browser_auto_rename(
        &mut self,
        queued: Option<crate::app::controller::state::runtime::PendingBrowserAutoRenameIntent>,
    ) {
        let Some(queued) = queued else {
            return;
        };
        if self.selected_source_id().as_ref() != Some(&queued.source_id) {
            self.set_status(
                "Queued auto rename skipped; source changed",
                StatusTone::Info,
            );
            return;
        }
        if let Err(err) = self
            .browser()
            .auto_rename_browser_sample_paths_action(&queued.paths)
        {
            self.set_status(
                format!("Queued auto rename failed: {err}"),
                StatusTone::Error,
            );
        }
    }
}

fn remap_queued_browser_auto_rename_path(
    queued: &mut Option<crate::app::controller::state::runtime::PendingBrowserAutoRenameIntent>,
    old_relative: &std::path::Path,
    new_relative: &std::path::Path,
) {
    let Some(queued) = queued.as_mut() else {
        return;
    };
    for path in &mut queued.paths {
        if path == old_relative {
            *path = new_relative.to_path_buf();
        }
    }
}
