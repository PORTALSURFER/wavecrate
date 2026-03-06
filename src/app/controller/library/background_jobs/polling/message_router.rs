//! Central dispatch from queued job messages into focused handlers.

use super::*;

impl AppController {
    /// Route one queued job message to its owning handler.
    pub(super) fn handle_background_job_message(&mut self, message: JobMessage) {
        match message {
            JobMessage::WavLoaded(message) => self.handle_wav_loaded_message(message),
            JobMessage::AudioLoaded(message) => self.handle_audio_loaded_message(message),
            JobMessage::RecordingWaveformLoaded(message) => {
                self.handle_recording_waveform_loaded_message(message)
            }
            JobMessage::Scan(message) => self.handle_scan_message(message),
            JobMessage::FolderScanFinished(message) => {
                self.handle_folder_scan_finished_message(message)
            }
            JobMessage::SourceWatch(message) => {
                self.handle_source_watch_event(&message.source_id);
            }
            JobMessage::TrashMove(message) => self.handle_trash_move_message(message),
            JobMessage::FolderDeleteRecoveryFinished(report) => {
                self.apply_folder_delete_recovery_report(report);
            }
            JobMessage::FileOps(message) => self.handle_file_ops_message(message),
            JobMessage::Analysis(message) => {
                analysis::handle_analysis_message(self, message);
            }
            JobMessage::AnalysisFailuresLoaded(message) => {
                self.handle_analysis_failures_loaded_message(message)
            }
            JobMessage::UmapBuilt(message) => self.handle_umap_built_message(message),
            JobMessage::UmapClustersBuilt(message) => {
                self.handle_umap_clusters_built_message(message)
            }
            JobMessage::SimilarityPrepared(message) => {
                similarity::handle_similarity_prepared(self, message);
            }
            JobMessage::UpdateChecked(message) => {
                updates::handle_update_checked(self, message);
            }
            JobMessage::IssueGatewayCreated(message) => {
                updates::handle_issue_gateway_created(self, message);
            }
            JobMessage::IssueGatewayAuthed(message) => {
                updates::handle_issue_gateway_authed(self, message);
            }
            JobMessage::IssueTokenLoaded(message) => {
                updates::handle_issue_token_loaded(self, message);
            }
            JobMessage::IssueTokenSaved(message) => {
                updates::handle_issue_token_saved(self, message);
            }
            JobMessage::IssueTokenDeleted(message) => {
                updates::handle_issue_token_deleted(self, message);
            }
            JobMessage::SourceDbMaintenanceFinished(message) => {
                self.handle_source_db_maintenance_finished_message(message)
            }
            JobMessage::BrowserSearchFinished(message) => {
                self.handle_browser_search_finished_message(message)
            }
            JobMessage::Normalized(message) => self.handle_normalized_message(message),
        }
    }
}
