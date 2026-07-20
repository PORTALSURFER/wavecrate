use radiant::prelude as ui;

use crate::native_app::app::{ClipboardHandoffTarget, GuiMessage, NativeAppState};
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;

impl NativeAppState {
    pub(super) fn apply_browser_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::ResizeFolder(message) => self.resize_folder_browser(message),
            GuiMessage::AddSourceDialogFinished(result) => {
                self.finish_add_source_dialog(result, context);
            }
            GuiMessage::FolderBrowser(message) => {
                if matches!(message, FolderBrowserMessage::SetCurationScope(_, _)) {
                    self.ui.chrome.curation_filter_dropdown_open = false;
                }
                if matches!(message, FolderBrowserMessage::SetHarvestFilter(_, _)) {
                    self.ui.chrome.harvest_filter_dropdown_open = false;
                }
                self.apply_folder_browser_message(message, context);
            }
            GuiMessage::SetSimilarityAspectWeightingEnabled(enabled) => {
                self.set_similarity_aspect_weighting_enabled(enabled);
            }
            GuiMessage::SetSimilarityAspectEnabled { aspect, enabled } => {
                self.set_similarity_aspect_enabled(aspect, enabled);
            }
            GuiMessage::SetSimilarityAspectWeight { aspect, weight } => {
                self.set_similarity_aspect_weight(aspect, weight);
            }
            GuiMessage::SimilaritySettingsPersisted(result) => {
                self.finish_similarity_settings_persist(result);
            }
            GuiMessage::StarmapLayoutLoaded(result) => {
                self.finish_starmap_layout_load(result);
            }
            GuiMessage::SimilarityScoresResolved(result) => {
                self.finish_similarity_scores(result, context);
            }
            GuiMessage::SimilarityReadinessAdvanced {
                source_id,
                lifecycle_generation,
            } => {
                if self.background.source_lifecycle_generations.get(&source_id)
                    != Some(&lifecycle_generation)
                {
                    return;
                }
                self.finish_similarity_readiness_advanced(source_id, context);
            }
            GuiMessage::SourceProcessingProgress(progress) => {
                let source_is_current = if progress.source_id.is_empty() {
                    !progress.active
                        || !self
                            .library
                            .folder_browser
                            .configured_sample_sources()
                            .is_empty()
                } else {
                    self.library
                        .folder_browser
                        .source_exists(&progress.source_id)
                        && self
                            .background
                            .source_lifecycle_generations
                            .get(&progress.source_id)
                            == Some(&progress.lifecycle_generation)
                };
                if !source_is_current {
                    return;
                }
                if !progress.active {
                    self.background.source_processing_progress = None;
                    self.ui.chrome.job_details_open = false;
                } else {
                    self.background.source_processing_progress = Some(progress);
                }
                context.repaint(ui::RepaintScope::Projection);
            }
            GuiMessage::FolderScanProgress(progress) => {
                self.apply_folder_scan_progress(progress);
            }
            GuiMessage::FolderScanDiscoveryBatch(batch) => {
                self.apply_folder_scan_discovery_batch(batch);
            }
            GuiMessage::FolderScanFinished(result) => self.finish_folder_scan(result, context),
            GuiMessage::FolderScanMaintenanceFinished(result) => {
                self.finish_folder_scan_maintenance(result)
            }
            GuiMessage::FolderTreeRefreshFinished(completion) => {
                self.finish_folder_tree_refresh(completion, context);
            }
            GuiMessage::SelectedFolderVerifyFinished(ticket) => {
                self.finish_folder_verify(ticket, context)
            }
            GuiMessage::SourceFilesystemChanged {
                source_id,
                paths,
                overflowed,
                source_root_available,
            } => {
                self.refresh_source_after_filesystem_change(
                    source_id,
                    paths,
                    overflowed,
                    source_root_available,
                    context,
                );
            }
            GuiMessage::SourceWatcherReady => {
                self.background
                    .source_processing
                    .request_manifest_audits("source_watcher_ready");
            }
            GuiMessage::SourceFilesystemSyncFinished(result) => {
                self.finish_source_filesystem_sync(result, context);
            }
            GuiMessage::SourceManifestAuditCommitted {
                source_id,
                lifecycle_generation,
                committed_delta,
            } => {
                self.finish_source_manifest_audit(
                    source_id,
                    lifecycle_generation,
                    committed_delta,
                    context,
                );
            }
            GuiMessage::NormalizationProgress(progress) => {
                self.apply_normalization_progress(progress);
            }
            GuiMessage::NormalizationFinished(result) => self.finish_normalization(result, context),
            GuiMessage::SelectSampleWithModifiers { path, modifiers } => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.ui.browser_interaction.context_menu = None;
                self.select_sample_with_modifiers(path, modifiers, context);
            }
            GuiMessage::OpenSampleContextMenu { path, position } => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.open_sample_context_menu(path, position);
            }
            GuiMessage::DragSampleFile { path, drag } => {
                self.ui.browser_interaction.clipboard_handoff_target =
                    ClipboardHandoffTarget::BrowserFiles;
                self.ui.browser_interaction.context_menu = None;
                self.drag_sample_file(path, drag, context);
            }
            GuiMessage::ExternalDragCompleted(result) => {
                self.external_drag_completed(result, context)
            }
            _ => unreachable!("browser dispatcher received a non-browser message"),
        }
    }
}
