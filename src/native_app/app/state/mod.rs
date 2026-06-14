mod audio;
mod background;
mod library;
mod metadata;
/// Background scan worker transport and batching for source scans.
mod source_scan_worker;
mod source_scan_workflow;
mod transactions;
mod ui_state;
mod waveform;

#[cfg(test)]
pub(in crate::native_app) const DEFAULT_VOLUME: f32 = 1.0;

pub(in crate::native_app) use audio::AudioAppState;
pub(in crate::native_app) use background::{
    AudioOpenCompletion, AudioOpenTaskCompletion, BackgroundTaskState,
};
pub(in crate::native_app) use library::LibraryAppState;
pub(in crate::native_app) use metadata::MetadataAppState;
pub(in crate::native_app) use source_scan_worker::run_folder_scan_worker;
pub(in crate::native_app) use source_scan_workflow::{
    SourceFilesystemChangePlan, SourceRefreshRequest, SourceScanFinish, SourceScanWorkflow,
};
pub(in crate::native_app) use transactions::TransactionState;
pub(in crate::native_app) use ui_state::{
    ChromeUiState, PendingFolderDelete, SettingsAppState, StartupState, StatusState, UiAppState,
};
pub(in crate::native_app) use waveform::WaveformAppState;

pub(in crate::native_app) struct NativeAppState {
    pub(in crate::native_app) ui: UiAppState,
    pub(in crate::native_app) library: LibraryAppState,
    pub(in crate::native_app) waveform: WaveformAppState,
    pub(in crate::native_app) background: BackgroundTaskState,
    pub(in crate::native_app) audio: AudioAppState,
    pub(in crate::native_app) transactions: TransactionState,
    pub(in crate::native_app) metadata: MetadataAppState,
}
