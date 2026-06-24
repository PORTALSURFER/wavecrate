use std::{collections::HashSet, path::PathBuf, time::Instant};

use radiant::prelude as ui;
use wavecrate::sample_sources::SampleSource;
use wavecrate::sample_sources::config::AppSettingsCore;
use wavecrate::selection::SelectionRange;

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, NativeFileDropHover};
use crate::native_app::sample_library::context_menu_target::BrowserContextMenu;

pub(in crate::native_app) const DEFAULT_BEAT_GUIDE_COUNT: u8 = 4;
pub(in crate::native_app) const MIN_BEAT_GUIDE_COUNT: u8 = 1;
pub(in crate::native_app) const MAX_BEAT_GUIDE_COUNT: u8 = 64;

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
    pub(in crate::native_app) shortcut_help_open: bool,
    pub(in crate::native_app) help_tooltips_enabled: bool,
    pub(in crate::native_app) sticky_random_sample_range_playback: bool,
    pub(in crate::native_app) beat_guides_enabled: bool,
    pub(in crate::native_app) beat_guide_count: u8,
}

impl ChromeUiState {
    pub(in crate::native_app) fn new(folder_width: f32) -> Self {
        Self {
            folder_panel: ui::PanelResizeState::new(folder_width),
            job_details_open: false,
            transaction_list_open: false,
            shortcut_help_open: false,
            help_tooltips_enabled: false,
            sticky_random_sample_range_playback: false,
            beat_guides_enabled: false,
            beat_guide_count: DEFAULT_BEAT_GUIDE_COUNT,
        }
    }

    pub(in crate::native_app) fn adjust_beat_guide_count(&mut self, delta: i8) {
        let next = i16::from(self.beat_guide_count) + i16::from(delta);
        self.beat_guide_count = next.clamp(
            i16::from(MIN_BEAT_GUIDE_COUNT),
            i16::from(MAX_BEAT_GUIDE_COUNT),
        ) as u8;
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
    pub(in crate::native_app) similarity_persist_deadline: Option<Instant>,
    pub(in crate::native_app) similarity_persist_inflight: bool,
}

impl SettingsAppState {
    pub(in crate::native_app) fn new(persisted: AppSettingsCore) -> Self {
        Self {
            persisted,
            ui: SettingsUiState::default(),
            similarity_persist_deadline: None,
            similarity_persist_inflight: false,
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
    pub(in crate::native_app) pending_waveform_destructive_edit:
        Option<PendingWaveformDestructiveEdit>,
    pub(in crate::native_app) cut_file_clipboard: Option<CutFileClipboard>,
    pub(in crate::native_app) cut_file_paste_task_id: Option<u64>,
    pub(in crate::native_app) native_file_drop_hover: Option<NativeFileDropHover>,
    pub(in crate::native_app) pending_internal_file_drag_paths: HashSet<PathBuf>,
    pub(in crate::native_app) file_move_conflict_apply_to_remaining: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct CutFileClipboard {
    pub(in crate::native_app) file_ids: Vec<String>,
}

impl CutFileClipboard {
    pub(in crate::native_app) fn new(file_ids: Vec<String>) -> Self {
        Self { file_ids }
    }

    pub(in crate::native_app) fn len(&self) -> usize {
        self.file_ids.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct PendingFolderDelete {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum ExtractedFilePlaybackType {
    OneShot,
    Loop,
}

impl ExtractedFilePlaybackType {
    pub(in crate::native_app) fn from_loop_active(loop_active: bool) -> Self {
        if loop_active {
            Self::Loop
        } else {
            Self::OneShot
        }
    }

    pub(in crate::native_app) fn tag(self) -> &'static str {
        match self {
            Self::OneShot => "one-shot",
            Self::Loop => "loop",
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct PendingWaveformDestructiveEdit {
    pub(in crate::native_app) prompt: WaveformDestructiveEditPrompt,
    pub(in crate::native_app) source: SampleSource,
    pub(in crate::native_app) relative_path: PathBuf,
    pub(in crate::native_app) absolute_path: PathBuf,
    pub(in crate::native_app) selection: SelectionRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformDestructiveEditKind {
    CropSelection,
    TrimSelection,
    ExtractAndTrimSelection,
    ApplyEditSelectionEffects,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct WaveformDestructiveEditPrompt {
    pub(in crate::native_app) edit: WaveformDestructiveEditKind,
    pub(in crate::native_app) title: String,
    pub(in crate::native_app) message: String,
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
