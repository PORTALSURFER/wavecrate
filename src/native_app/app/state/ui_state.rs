use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    time::Instant,
};

use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use wavecrate::sample_sources::SampleSource;
use wavecrate::sample_sources::config::AppSettingsCore;
use wavecrate::selection::SelectionRange;

use crate::native_app::app::{AppSettingsTab, AudioSettingsDropdown, NativeFileDropHover};
use crate::native_app::sample_library::context_menu_target::BrowserContextMenu;
use crate::native_app::waveform::WaveformContextMenu;

pub(in crate::native_app) const DEFAULT_BEAT_GUIDE_COUNT: u8 = 4;
pub(in crate::native_app) const MIN_BEAT_GUIDE_COUNT: u8 = 1;
pub(in crate::native_app) const MAX_BEAT_GUIDE_COUNT: u8 = 64;

pub(in crate::native_app) struct UiAppState {
    pub(in crate::native_app) chrome: ChromeUiState,
    pub(in crate::native_app) status: StatusState,
    pub(in crate::native_app) release_update: ReleaseUpdateState,
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
            release_update: ReleaseUpdateState::default(),
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
    pub(in crate::native_app) sample_browser_display: SampleBrowserDisplayMode,
    pub(in crate::native_app) sample_map_viewport: SampleMapViewport,
    pub(in crate::native_app) sample_map_audition_drag: Option<SampleMapAuditionDragState>,
    pub(in crate::native_app) sample_map_audition_queue: SampleMapAuditionQueueState,
    pub(in crate::native_app) harvest_family_open: bool,
    pub(in crate::native_app) beat_guides_enabled: bool,
    pub(in crate::native_app) beat_guide_count: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SampleBrowserDisplayMode {
    List,
    Map,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SampleMapAuditionDragState {
    pub(in crate::native_app) last_hit_file_id: Option<String>,
    pub(in crate::native_app) last_position: ui::Point,
    pub(in crate::native_app) modifiers: PointerModifiers,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct SampleMapAuditionQueueState {
    pub(in crate::native_app) active_file_id: Option<String>,
    pub(in crate::native_app) queued_file_ids: VecDeque<String>,
    pub(in crate::native_app) seen_file_ids: HashSet<String>,
    pub(in crate::native_app) modifiers: PointerModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct SampleMapViewport {
    pub(in crate::native_app) center_x: f32,
    pub(in crate::native_app) center_y: f32,
    pub(in crate::native_app) zoom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum SampleMapViewportChange {
    Pan { delta: ui::Vector2 },
    Zoom { anchor: ui::Vector2, factor: f32 },
    Center { x: f32, y: f32 },
    Reset,
}

impl Default for SampleMapViewport {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            zoom: 1.0,
        }
    }
}

impl SampleMapViewport {
    const MIN_ZOOM: f32 = 1.0;
    const MAX_ZOOM: f32 = 8.0;

    pub(in crate::native_app) fn apply_change(&mut self, change: SampleMapViewportChange) {
        match change {
            SampleMapViewportChange::Pan { delta } => {
                self.center_x -= delta.x / self.zoom;
                self.center_y -= delta.y / self.zoom;
                self.clamp_center();
            }
            SampleMapViewportChange::Zoom { anchor, factor } => {
                self.zoom_at(anchor, factor);
            }
            SampleMapViewportChange::Center { x, y } => {
                self.center_x = x;
                self.center_y = y;
                self.clamp_center();
            }
            SampleMapViewportChange::Reset => *self = Self::default(),
        }
    }

    fn zoom_at(&mut self, anchor: ui::Vector2, factor: f32) {
        let previous_zoom = self.zoom;
        let next_zoom = (self.zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        if (next_zoom - previous_zoom).abs() <= f32::EPSILON {
            return;
        }
        let world_x = self.center_x + (anchor.x - 0.5) / previous_zoom;
        let world_y = self.center_y + (anchor.y - 0.5) / previous_zoom;
        self.zoom = next_zoom;
        self.center_x = world_x - (anchor.x - 0.5) / next_zoom;
        self.center_y = world_y - (anchor.y - 0.5) / next_zoom;
        self.clamp_center();
    }

    fn clamp_center(&mut self) {
        let half_span = 0.5 / self.zoom;
        self.center_x = self.center_x.clamp(half_span, 1.0 - half_span);
        self.center_y = self.center_y.clamp(half_span, 1.0 - half_span);
    }
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
            sample_browser_display: SampleBrowserDisplayMode::List,
            sample_map_viewport: SampleMapViewport::default(),
            sample_map_audition_drag: None,
            sample_map_audition_queue: SampleMapAuditionQueueState::default(),
            harvest_family_open: false,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct ReleaseUpdateState {
    pub(in crate::native_app) status: ReleaseUpdateStatus,
    pub(in crate::native_app) latest: Option<wavecrate::updater::PublicReleaseInfo>,
    pub(in crate::native_app) last_error: Option<String>,
}

impl ReleaseUpdateState {
    pub(in crate::native_app) fn begin_check(&mut self) {
        self.status = ReleaseUpdateStatus::Checking;
        self.latest = None;
        self.last_error = None;
    }

    pub(in crate::native_app) fn finish(
        &mut self,
        result: Result<Option<wavecrate::updater::PublicReleaseInfo>, String>,
    ) {
        match result {
            Ok(Some(release)) => {
                self.status = ReleaseUpdateStatus::Available;
                self.latest = Some(release);
                self.last_error = None;
            }
            Ok(None) => {
                self.status = ReleaseUpdateStatus::UpToDate;
                self.latest = None;
                self.last_error = None;
            }
            Err(error) => {
                self.status = ReleaseUpdateStatus::Error;
                self.latest = None;
                self.last_error = Some(error);
            }
        }
    }

    pub(in crate::native_app) fn available(&self) -> bool {
        self.status == ReleaseUpdateStatus::Available && self.latest.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum ReleaseUpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Error,
}

#[derive(Default)]
pub(in crate::native_app) struct BrowserInteractionState {
    pub(in crate::native_app) context_menu: Option<BrowserContextMenu>,
    pub(in crate::native_app) waveform_context_menu: Option<WaveformContextMenu>,
    pub(in crate::native_app) clipboard_handoff_target: ClipboardHandoffTarget,
    pub(in crate::native_app) pending_folder_delete: Option<PendingFolderDelete>,
    pub(in crate::native_app) pending_waveform_destructive_edit:
        Option<PendingWaveformDestructiveEdit>,
    pub(in crate::native_app) cut_file_clipboard: Option<CutFileClipboard>,
    pub(in crate::native_app) cut_file_paste_task_id: Option<u64>,
    pub(in crate::native_app) native_file_drop_hover: Option<NativeFileDropHover>,
    pub(in crate::native_app) pending_internal_file_drag_paths: HashSet<PathBuf>,
    pub(in crate::native_app) file_move_conflict_apply_to_remaining: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum ClipboardHandoffTarget {
    #[default]
    BrowserFiles,
    WaveformSelection,
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
    ReverseSelection,
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
    pub(in crate::native_app) app_icon_install_pending: bool,
    pub(in crate::native_app) release_update_check_pending: bool,
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
            app_icon_install_pending: true,
            release_update_check_pending: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_map_viewport_zoom_keeps_anchor_world_position_stable() {
        let mut viewport = SampleMapViewport::default();
        let anchor = ui::Vector2::new(0.25, 0.75);
        let before = (
            viewport.center_x + (anchor.x - 0.5) / viewport.zoom,
            viewport.center_y + (anchor.y - 0.5) / viewport.zoom,
        );

        viewport.apply_change(SampleMapViewportChange::Zoom {
            anchor,
            factor: 2.0,
        });

        let after = (
            viewport.center_x + (anchor.x - 0.5) / viewport.zoom,
            viewport.center_y + (anchor.y - 0.5) / viewport.zoom,
        );
        assert!((before.0 - after.0).abs() < 0.0001);
        assert!((before.1 - after.1).abs() < 0.0001);
        assert_eq!(viewport.zoom, 2.0);
    }

    #[test]
    fn sample_map_viewport_pan_is_scaled_by_zoom_and_clamped() {
        let mut viewport = SampleMapViewport {
            center_x: 0.5,
            center_y: 0.5,
            zoom: 2.0,
        };

        viewport.apply_change(SampleMapViewportChange::Pan {
            delta: ui::Vector2::new(0.2, -0.1),
        });

        assert!((viewport.center_x - 0.4).abs() < 0.0001);
        assert!((viewport.center_y - 0.55).abs() < 0.0001);

        viewport.apply_change(SampleMapViewportChange::Pan {
            delta: ui::Vector2::new(100.0, 100.0),
        });
        assert_eq!(viewport.center_x, 0.25);
        assert_eq!(viewport.center_y, 0.25);
    }

    #[test]
    fn sample_map_viewport_center_clamps_to_current_zoom_bounds() {
        let mut viewport = SampleMapViewport {
            center_x: 0.5,
            center_y: 0.5,
            zoom: 4.0,
        };

        viewport.apply_change(SampleMapViewportChange::Center { x: 0.05, y: 0.95 });

        assert_eq!(viewport.center_x, 0.125);
        assert_eq!(viewport.center_y, 0.875);
    }
}
