mod browser;
mod chrome;
mod files;
mod frame;
mod metadata;
mod navigation;
mod playback;
mod sample_loading;
mod settings;
#[cfg(test)]
mod tests;
mod waveform;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(in crate::native_app) fn handle_message(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.apply_message(message, context);
    }

    pub(in crate::native_app) fn apply_message(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::ResizeFolder(_)
            | GuiMessage::AddSourceDialogFinished(_)
            | GuiMessage::FolderBrowser(_)
            | GuiMessage::PrepareSimilarityForSelectedSource
            | GuiMessage::SetSimilarityAspectWeightingEnabled(_)
            | GuiMessage::SetSimilarityAspectEnabled { .. }
            | GuiMessage::SetSimilarityAspectWeight { .. }
            | GuiMessage::SimilaritySettingsPersisted(_)
            | GuiMessage::SimilarityPrepStatusResolved(_)
            | GuiMessage::SimilarityPrepEnqueueFinished(_)
            | GuiMessage::SimilarityScoresResolved(_)
            | GuiMessage::FolderScanProgress(_)
            | GuiMessage::FolderScanDiscoveryBatch(_)
            | GuiMessage::FolderScanFinished(_)
            | GuiMessage::FolderTreeRefreshFinished(_)
            | GuiMessage::SelectedFolderVerifyFinished(_)
            | GuiMessage::SourceFilesystemChanged { .. }
            | GuiMessage::NormalizationProgress(_)
            | GuiMessage::NormalizationFinished(_)
            | GuiMessage::SelectSampleWithModifiers { .. }
            | GuiMessage::OpenSampleContextMenu { .. }
            | GuiMessage::DragSampleFile { .. }
            | GuiMessage::ExternalDragCompleted(_) => self.apply_browser_dispatch(message, context),
            GuiMessage::DeferredSampleLoad { .. }
            | GuiMessage::SampleLoadProgress(_, _, _)
            | GuiMessage::SamplePlaybackReady(_)
            | GuiMessage::SampleLoadFinished(_)
            | GuiMessage::WaveformCacheIndicatorRefreshFinished(_)
            | GuiMessage::WaveformCacheWarmFinished(_)
            | GuiMessage::ActiveFolderCacheWarmPlanned(_)
            | GuiMessage::ActiveFolderCacheWarmReady(_)
            | GuiMessage::ActiveFolderCacheWarmProgress(_)
            | GuiMessage::ActiveFolderCacheWarmFinished(_) => {
                self.apply_sample_loading_dispatch(message, context);
            }
            GuiMessage::AudioPlayerOpenFinished(_)
            | GuiMessage::PlaySelectedSample
            | GuiMessage::PlayRandomSampleRange
            | GuiMessage::LastPlayedPersistReady { .. }
            | GuiMessage::LastPlayedPersisted(_)
            | GuiMessage::VolumeSettingsPersisted(_)
            | GuiMessage::StopPlayback
            | GuiMessage::ToggleLoopPlayback => self.apply_playback_dispatch(message, context),
            GuiMessage::Settings(message) => self.apply_settings_message(message, context),
            GuiMessage::Metadata(message) => self.apply_metadata_message(message, context),
            GuiMessage::FocusLoadedFile
            | GuiMessage::AdjustSelectedRating(_)
            | GuiMessage::AssignSelectedCollection(_)
            | GuiMessage::RemoveContextSampleFromCollection
            | GuiMessage::NormalizeSelectedSamples
            | GuiMessage::CopySelectedFiles
            | GuiMessage::SelectedFilesCopyFinished { .. }
            | GuiMessage::SetFileMoveConflictApplyToRemaining(_)
            | GuiMessage::ResolveFileMoveConflict(_)
            | GuiMessage::FolderMoveFinished { .. }
            | GuiMessage::FileMoveConflictFinished { .. }
            | GuiMessage::CancelFileMoveConflicts
            | GuiMessage::CopyContextPath
            | GuiMessage::TrashFolderDialogFinished(_)
            | GuiMessage::ContextPathCopyFinished { .. }
            | GuiMessage::OpenContextTarget
            | GuiMessage::CreateFolderAtContextTarget
            | GuiMessage::ContextFolderCreateFinished { .. }
            | GuiMessage::MoveContextTargetToTrash
            | GuiMessage::RequestDeleteContextFolder
            | GuiMessage::ConfirmContextFolderDelete
            | GuiMessage::CancelContextFolderDelete
            | GuiMessage::TrashMoveFinished { .. }
            | GuiMessage::ContextTargetOpenFinished { .. }
            | GuiMessage::RefreshContextSource
            | GuiMessage::RemoveContextSource
            | GuiMessage::CloseContextMenu
            | GuiMessage::ExternalWaveformFileDropFinished { .. }
            | GuiMessage::WaveformFileDrop(_) => self.apply_file_dispatch(message, context),
            GuiMessage::ToggleJobDetails
            | GuiMessage::CloseJobDetails
            | GuiMessage::UndoTransaction
            | GuiMessage::RedoTransaction
            | GuiMessage::ToggleTransactionList
            | GuiMessage::CloseTransactionList
            | GuiMessage::FocusRenameInput(_)
            | GuiMessage::FolderBrowserRenameFinished(_)
            | GuiMessage::DeleteSelectedItem
            | GuiMessage::ExtractPlaymarkedRange
            | GuiMessage::PlaySelectionExtractionFinished { .. } => {
                self.apply_chrome_dispatch(message, context);
            }
            GuiMessage::NavigateBrowser { .. }
            | GuiMessage::ToggleSelectedSampleAndAdvance
            | GuiMessage::SelectAllSamples
            | GuiMessage::ToggleRandomNavigationMode
            | GuiMessage::SampleBrowserWindowChanged(_)
            | GuiMessage::FolderTreeWindowChanged(_)
            | GuiMessage::CollapseSelectedFolder
            | GuiMessage::ExpandSelectedFolder
            | GuiMessage::CancelBrowserDragOnSampleList
            | GuiMessage::DropWaveformSelectionOnSampleList => {
                self.apply_navigation_dispatch(message, context);
            }
            GuiMessage::Waveform(message) => self.apply_waveform_message(message, context),
            GuiMessage::Frame => self.apply_frame_message(context),
        }
    }
}
