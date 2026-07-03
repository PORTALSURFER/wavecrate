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
pub(in crate::native_app) mod waveform;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, WaveformInteraction};

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
        if closes_waveform_context_menu(&message) {
            self.ui.browser_interaction.waveform_context_menu = None;
        }
        match message {
            GuiMessage::ResizeFolder(_)
            | GuiMessage::AddSourceDialogFinished(_)
            | GuiMessage::FolderBrowser(_)
            | GuiMessage::SetSimilarityAspectWeightingEnabled(_)
            | GuiMessage::SetSimilarityAspectEnabled { .. }
            | GuiMessage::SetSimilarityAspectWeight { .. }
            | GuiMessage::SimilaritySettingsPersisted(_)
            | GuiMessage::StarmapLayoutLoaded(_)
            | GuiMessage::SimilarityPrepStatusResolved(_)
            | GuiMessage::SimilarityPrepEnqueueFinished(_)
            | GuiMessage::SimilarityScoresResolved(_)
            | GuiMessage::FolderScanProgress(_)
            | GuiMessage::FolderScanDiscoveryBatch(_)
            | GuiMessage::FolderScanFinished(_)
            | GuiMessage::FolderTreeRefreshFinished(_)
            | GuiMessage::SelectedFolderVerifyFinished(_)
            | GuiMessage::SourceFilesystemChanged { .. }
            | GuiMessage::SourceFilesystemSyncFinished(_)
            | GuiMessage::NormalizationProgress(_)
            | GuiMessage::NormalizationFinished(_)
            | GuiMessage::SelectSampleWithModifiers { .. }
            | GuiMessage::OpenSampleContextMenu { .. }
            | GuiMessage::RememberBrowserContextMenuPointerAnchor(_)
            | GuiMessage::DragSampleFile { .. }
            | GuiMessage::ExternalDragCompleted(_) => self.apply_browser_dispatch(message, context),
            GuiMessage::DeferredSampleLoad { .. }
            | GuiMessage::SampleLoadPathValidated { .. }
            | GuiMessage::SampleLoadProgress(_, _, _)
            | GuiMessage::SamplePlaybackReady(_)
            | GuiMessage::SampleLoadFinished(_)
            | GuiMessage::WaveformCacheIndicatorRefreshFinished(_)
            | GuiMessage::WaveformCacheWarmFinished(_)
            | GuiMessage::ActiveFolderCacheWarmPlanProgress(_)
            | GuiMessage::ActiveFolderCacheWarmPlanned(_)
            | GuiMessage::ActiveFolderCacheWarmReady(_)
            | GuiMessage::ActiveFolderCacheWarmProgress(_)
            | GuiMessage::ActiveFolderCacheWarmFinished(_) => {
                self.apply_sample_loading_dispatch(message, context);
            }
            GuiMessage::AudioPlayerOpenFinished(_)
            | GuiMessage::PlaySelectedSample
            | GuiMessage::PlayFromCurrentPlayStart
            | GuiMessage::PlayRandomSampleRange
            | GuiMessage::PlayRandomListedSampleRange
            | GuiMessage::PlayPreviousPlaybackHistory
            | GuiMessage::PlayNextPlaybackHistory
            | GuiMessage::LastPlayedPersistReady { .. }
            | GuiMessage::LastPlayedPersisted(_)
            | GuiMessage::HarvestSeenPersisted(_)
            | GuiMessage::VolumeSettingsPersisted(_)
            | GuiMessage::StopPlayback
            | GuiMessage::ToggleLoopPlayback
            | GuiMessage::ToggleMetronome => self.apply_playback_dispatch(message, context),
            GuiMessage::Settings(message) => self.apply_settings_message(message, context),
            GuiMessage::Metadata(message) => self.apply_metadata_message(message, context),
            GuiMessage::FocusLoadedFile
            | GuiMessage::AdjustSelectedRatingWithoutAdvance(_)
            | GuiMessage::AssignSelectedCollection(_)
            | GuiMessage::RemoveContextSampleFromCollection
            | GuiMessage::CleanMissingContextSampleFromCollection
            | GuiMessage::CleanMissingFilesFromActiveCollection
            | GuiMessage::MarkContextSampleHarvestDone
            | GuiMessage::MarkContextSampleHarvestIgnored
            | GuiMessage::ResetContextSampleHarvest
            | GuiMessage::ToggleSelectedHarvestDone
            | GuiMessage::ShowContextSampleHarvestOrigin
            | GuiMessage::ShowContextSampleHarvestDerivatives
            | GuiMessage::OpenContextSampleHarvestDestination
            | GuiMessage::ShowSelectedSampleHarvestOrigin
            | GuiMessage::ShowSelectedSampleHarvestDerivatives
            | GuiMessage::OpenSelectedSampleHarvestDestination
            | GuiMessage::NormalizeSelectedSamples
            | GuiMessage::CopySelectedFiles
            | GuiMessage::CutSelectedFiles
            | GuiMessage::PasteCutFiles
            | GuiMessage::DuplicateContextSampleSame
            | GuiMessage::DuplicateContextSampleDouble
            | GuiMessage::ContextSampleSameFinished { .. }
            | GuiMessage::ContextSampleDoubleFinished { .. }
            | GuiMessage::SelectedFilesCopyFinished { .. }
            | GuiMessage::WaveformSelectionCopyExtracted { .. }
            | GuiMessage::WaveformSelectionCopyFinished { .. }
            | GuiMessage::FileMoveProgress(_)
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
            | GuiMessage::RenameContextFolder
            | GuiMessage::ContextFolderCreateFinished { .. }
            | GuiMessage::MoveContextTargetToTrash
            | GuiMessage::UnlockContextSample
            | GuiMessage::ToggleContextFolderLock
            | GuiMessage::RequestDeleteContextFolder
            | GuiMessage::ConfirmContextFolderDelete
            | GuiMessage::CancelContextFolderDelete
            | GuiMessage::TrashMoveFinished { .. }
            | GuiMessage::ContextTargetOpenFinished { .. }
            | GuiMessage::RefreshContextSource
            | GuiMessage::ProcessContextSource
            | GuiMessage::ToggleContextSourceProtection
            | GuiMessage::SetContextSourcePrimary
            | GuiMessage::ClearContextSourcePrimary
            | GuiMessage::RemoveContextSource
            | GuiMessage::CloseContextMenu
            | GuiMessage::ExternalWaveformFileDropFinished { .. }
            | GuiMessage::NativeAudioDocumentOpenValidated { .. }
            | GuiMessage::WaveformFileDrop(_) => self.apply_file_dispatch(message, context),
            GuiMessage::ToggleJobDetails
            | GuiMessage::CloseJobDetails
            | GuiMessage::ReleaseUpdateCheckFinished(_)
            | GuiMessage::OpenReleaseDownloadPage
            | GuiMessage::ToggleShortcutHelp
            | GuiMessage::CloseShortcutHelp
            | GuiMessage::ToggleStickyRandomSampleRangePlayback
            | GuiMessage::ToggleCurationFilterDropdown
            | GuiMessage::CloseCurationFilterDropdown
            | GuiMessage::ToggleHarvestFilterDropdown
            | GuiMessage::CloseHarvestFilterDropdown
            | GuiMessage::ToggleZeroCrossingSnap
            | GuiMessage::ToggleBeatGuides
            | GuiMessage::SetBeatGuideCount(_)
            | GuiMessage::ChangeBeatGuideCountInput(_)
            | GuiMessage::CommitBeatGuideCountInput(_)
            | GuiMessage::ToggleSimilarSections
            | GuiMessage::SimilarSectionsResolved(_)
            | GuiMessage::UndoTransaction
            | GuiMessage::RedoTransaction
            | GuiMessage::UndoTransactionsThrough(_)
            | GuiMessage::RedoTransactionsThrough(_)
            | GuiMessage::ToggleTransactionList
            | GuiMessage::CloseTransactionList
            | GuiMessage::FocusRenameInput(_)
            | GuiMessage::FolderBrowserRenameFinished(_)
            | GuiMessage::DeleteSelectedItem
            | GuiMessage::RequestCropWaveformSelection
            | GuiMessage::RequestTrimWaveformSelection
            | GuiMessage::RequestReverseWaveformSelection
            | GuiMessage::RequestMuteWaveformSelection
            | GuiMessage::RequestExtractAndTrimWaveformSelection
            | GuiMessage::RequestCropPlaymarkSelection
            | GuiMessage::RequestTrimPlaymarkSelection
            | GuiMessage::RequestReversePlaymarkSelection
            | GuiMessage::RequestExtractAndTrimPlaymarkSelection
            | GuiMessage::RequestApplyEditSelectionEffects
            | GuiMessage::OpenContextMenu
            | GuiMessage::ConfirmPendingWaveformDestructiveEdit
            | GuiMessage::CancelPendingWaveformDestructiveEdit
            | GuiMessage::AddProtectedExtractionTargetSource
            | GuiMessage::ProtectedExtractionTargetSourceDialogFinished(_)
            | GuiMessage::CancelProtectedExtractionTargetSource
            | GuiMessage::WaveformDestructiveEditFinished(_)
            | GuiMessage::ExtractPlaymarkedRange
            | GuiMessage::ExtractPlaymarkedRangeToHarvestDestination
            | GuiMessage::PlaySelectionExtractionFinished { .. }
            | GuiMessage::SelectedWholeFilesHarvestExtractionFinished { .. } => {
                self.apply_chrome_dispatch(message, context);
            }
            GuiMessage::NavigateBrowser { .. }
            | GuiMessage::ToggleSelectedSampleAndAdvance
            | GuiMessage::SelectAllSamples
            | GuiMessage::ToggleRandomNavigationMode
            | GuiMessage::ToggleSampleBrowserMapView
            | GuiMessage::FocusSelectedStarmapNode
            | GuiMessage::ChangeStarmapViewport(_)
            | GuiMessage::BeginStarmapAuditionDrag { .. }
            | GuiMessage::UpdateStarmapAuditionDrag { .. }
            | GuiMessage::AdvanceStarmapAudition { .. }
            | GuiMessage::FinishStarmapAuditionDrag
            | GuiMessage::SampleBrowserWindowChanged(_)
            | GuiMessage::FolderTreeWindowChanged(_)
            | GuiMessage::CollapseSelectedFolder
            | GuiMessage::CancelBrowserDragOnSampleList
            | GuiMessage::DropWaveformSelectionOnSampleList => {
                self.apply_navigation_dispatch(message, context);
            }
            GuiMessage::Waveform(message) => self.apply_waveform_message(message, context),
            GuiMessage::Frame => self.apply_frame_message(context),
        }
    }
}

fn closes_waveform_context_menu(message: &GuiMessage) -> bool {
    matches!(
        message,
        GuiMessage::PlaySelectedSample
            | GuiMessage::ExtractPlaymarkedRange
            | GuiMessage::ExtractPlaymarkedRangeToHarvestDestination
            | GuiMessage::RequestCropPlaymarkSelection
            | GuiMessage::RequestTrimPlaymarkSelection
            | GuiMessage::RequestReversePlaymarkSelection
            | GuiMessage::RequestExtractAndTrimPlaymarkSelection
            | GuiMessage::ToggleSimilarSections
            | GuiMessage::Waveform(WaveformInteraction::ZoomToPlaySelection)
    )
}
