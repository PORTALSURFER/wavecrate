use sempal::updater::{ApplyPlan, ReleaseSummary, UpdateChannel, UpdateProgress, UpdaterRunArgs};
use std::sync::mpsc::Receiver;

/// One selectable release entry shown in the updater browser list.
#[derive(Debug, Clone)]
pub(super) struct ReleaseOption {
    pub(super) tag: String,
    pub(super) label: String,
    pub(super) html_url: String,
}

/// Background loading state for the release list.
#[derive(Debug, Clone)]
pub(super) enum ReleaseState {
    Idle,
    Loading,
    Loaded(Vec<ReleaseOption>),
    Error(String),
}

/// High-level updater status shown in the companion UI.
#[derive(Debug, Clone)]
pub(super) enum UiStatus {
    Idle,
    Updating,
    Success(String),
    Error(String),
}

/// Native bridge state for the standalone updater helper window.
pub(super) struct UpdateNativeBridge {
    pub(super) args: UpdaterRunArgs,
    pub(super) release_state: ReleaseState,
    pub(super) release_rx: Option<Receiver<Result<Vec<ReleaseSummary>, String>>>,
    pub(super) selected_tag: Option<String>,
    pub(super) status: UiStatus,
    pub(super) log: Vec<String>,
    pub(super) progress_rx: Option<Receiver<UpdateProgress>>,
    pub(super) result_rx: Option<Receiver<Result<ApplyPlan, String>>>,
    pub(super) show_log_view: bool,
}

impl UpdateNativeBridge {
    pub(super) fn new(args: UpdaterRunArgs) -> Self {
        let mut bridge = Self {
            args,
            release_state: ReleaseState::Idle,
            release_rx: None,
            selected_tag: None,
            status: UiStatus::Idle,
            log: Vec::new(),
            progress_rx: None,
            result_rx: None,
            show_log_view: false,
        };
        bridge.refresh_release_list();
        bridge
    }

    pub(super) fn ensure_selected_tag(&mut self) {
        if self.selected_tag.is_some() {
            return;
        }
        if let ReleaseState::Loaded(options) = &self.release_state
            && let Some(first) = options.first()
        {
            self.selected_tag = Some(first.tag.clone());
        }
    }

    pub(super) fn selected_release(&self) -> Option<&ReleaseOption> {
        let selected = self.selected_tag.as_deref()?;
        let ReleaseState::Loaded(options) = &self.release_state else {
            return None;
        };
        options.iter().find(|option| option.tag == selected)
    }

    pub(super) fn select_release_by_row(&mut self, visible_row: usize) {
        let ReleaseState::Loaded(options) = &self.release_state else {
            return;
        };
        if let Some(option) = options.get(visible_row) {
            self.selected_tag = Some(option.tag.clone());
        }
    }

    pub(super) fn move_release_focus(&mut self, delta: i8) {
        let ReleaseState::Loaded(options) = &self.release_state else {
            return;
        };
        if options.is_empty() {
            return;
        }
        let current_index = self
            .selected_tag
            .as_ref()
            .and_then(|tag| options.iter().position(|option| option.tag == *tag))
            .unwrap_or(0);
        let max_index = options.len() - 1;
        let next_index = if delta.is_negative() {
            current_index.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            (current_index + delta as usize).min(max_index)
        };
        if let Some(option) = options.get(next_index) {
            self.selected_tag = Some(option.tag.clone());
        }
    }
}

pub(super) fn channel_label(channel: UpdateChannel) -> &'static str {
    match channel {
        UpdateChannel::Stable => "stable",
        UpdateChannel::Nightly => "nightly",
    }
}
