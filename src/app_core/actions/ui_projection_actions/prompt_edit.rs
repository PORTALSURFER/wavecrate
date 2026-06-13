use serde::{Deserialize, Serialize};

use super::BrowserTagTarget;

/// Prompt, rename, file edit, and confirmation actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptEditAction {
    SetPromptInput { value: String },
    StartBrowserRename,
    ConfirmBrowserRename,
    CancelBrowserRename,
    AutoRenameBrowserSelection { visible_row: Option<usize> },
    TagBrowserSelection { target: BrowserTagTarget },
    DeleteBrowserSelection,
    NormalizeFocusedBrowserSample,
    NormalizeWaveformSelectionOrSample,
    CropWaveformSelection,
    CropWaveformSelectionToNewSample,
    TrimWaveformSelection,
    ReverseWaveformSelection,
    FadeWaveformSelectionLeftToRight,
    FadeWaveformSelectionRightToLeft,
    MuteWaveformSelection,
    DeleteSelectedSliceMarkers,
    ToggleWaveformSliceSelection { index: usize },
    AuditionWaveformDuplicateSlice { index: usize },
    ToggleWaveformDuplicateSliceExemption { index: usize },
    MoveWaveformSliceFocus { delta: i8 },
    ToggleFocusedWaveformSliceExportMark,
    AlignWaveformStartToMarker,
    DeleteLoadedWaveformSample,
    SlideWaveformSelection { delta: i8, fine: bool },
    ConfirmPrompt,
    CancelPrompt,
    CancelProgress,
    CopySelectionToClipboard,
    ToggleHotkeyOverlay,
    CopyStatusLog,
    OpenFeedbackIssuePrompt,
    MoveTrashedSamplesToFolder,
}
