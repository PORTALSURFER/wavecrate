use std::{collections::HashSet, path::PathBuf};

use radiant::prelude as ui;
use wavecrate::sample_sources::config::AppSettingsCore;

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, NativeFileDropHover};
use crate::native_app::sample_library::context_menu_target::BrowserContextMenu;

pub(in crate::native_app) struct UiAppState {
    pub(in crate::native_app) chrome: ChromeUiState,
    pub(in crate::native_app) status: StatusState,
    pub(in crate::native_app) settings: SettingsAppState,
    pub(in crate::native_app) browser_interaction: BrowserInteractionState,
    pub(in crate::native_app) startup: StartupState,
}

impl UiAppState {
    pub(in crate::native_app) fn new(
        chrome: ChromeUiState,
        status: StatusState,
        settings: SettingsAppState,
        startup: StartupState,
    ) -> Self {
        Self {
            chrome,
            status,
            settings,
            browser_interaction: BrowserInteractionState::default(),
            startup,
        }
    }
}

pub(in crate::native_app) struct ChromeUiState {
    pub(in crate::native_app) folder_panel: ui::PanelResizeState,
    pub(in crate::native_app) job_details_open: bool,
    pub(in crate::native_app) transaction_list_open: bool,
}

impl ChromeUiState {
    pub(in crate::native_app) fn new(folder_width: f32) -> Self {
        Self {
            folder_panel: ui::PanelResizeState::new(folder_width),
            job_details_open: false,
            transaction_list_open: false,
        }
    }
}

pub(in crate::native_app) struct SettingsUiState {
    pub(in crate::native_app) audio_settings_open: bool,
    pub(in crate::native_app) app_settings_tab: AppSettingsTab,
    pub(in crate::native_app) audio_settings_dropdown: ui::ExclusiveOpen<AudioSettingsDropdown>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            audio_settings_open: false,
            app_settings_tab: Default::default(),
            audio_settings_dropdown: ui::ExclusiveOpen::new(),
        }
    }
}

pub(in crate::native_app) struct SettingsAppState {
    pub(in crate::native_app) persisted: AppSettingsCore,
    pub(in crate::native_app) ui: SettingsUiState,
}

impl SettingsAppState {
    pub(in crate::native_app) fn new(persisted: AppSettingsCore) -> Self {
        Self {
            persisted,
            ui: SettingsUiState::default(),
        }
    }
}

pub(in crate::native_app) struct StatusState {
    pub(in crate::native_app) sample: String,
}

impl StatusState {
    pub(in crate::native_app) fn new(sample: impl Into<String>) -> Self {
        Self {
            sample: sample.into(),
        }
    }
}

#[derive(Default)]
pub(in crate::native_app) struct BrowserInteractionState {
    pub(in crate::native_app) context_menu: Option<BrowserContextMenu>,
    pub(in crate::native_app) pending_folder_delete: Option<PendingFolderDelete>,
    pub(in crate::native_app) native_file_drop_hover: Option<NativeFileDropHover>,
    pub(in crate::native_app) pending_internal_file_drag_paths: HashSet<PathBuf>,
    pub(in crate::native_app) file_move_conflict_apply_to_remaining: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct PendingFolderDelete {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) name: String,
}

pub(in crate::native_app) struct StartupState {
    pub(in crate::native_app) source_scan_pending: bool,
    pub(in crate::native_app) folder_verify_pending: bool,
    pub(in crate::native_app) auto_load_pending: bool,
}

impl StartupState {
    pub(in crate::native_app) fn new(
        source_scan_pending: bool,
        folder_verify_pending: bool,
        auto_load_pending: bool,
    ) -> Self {
        Self {
            source_scan_pending,
            folder_verify_pending,
            auto_load_pending,
        }
    }
}
