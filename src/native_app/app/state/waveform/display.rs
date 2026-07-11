use std::path::{Path, PathBuf};

use crate::native_app::waveform::{
    InstantWaveformPreview, InstantWaveformPreviewTier, WaveformState,
};

use super::{
    PendingPlaySelectionRetargetCycle, WaveformCacheState, WaveformEditSelectionSnapshot,
    WaveformLoadState, WaveformPlaySelectionSnapshot,
};

pub(in crate::native_app) struct WaveformAppState {
    pub(in crate::native_app) current: WaveformState,
    pub(in crate::native_app) display: WaveformDisplayState,
    pub(in crate::native_app) starmap_drag_restore: Option<WaveformState>,
    pub(in crate::native_app) load: WaveformLoadState,
    pub(in crate::native_app) cache: WaveformCacheState,
    pub(in crate::native_app) pending_play_selection_transaction:
        Option<WaveformPlaySelectionSnapshot>,
    pub(in crate::native_app) pending_edit_fade_transaction: Option<WaveformEditSelectionSnapshot>,
    pub(in crate::native_app) pending_edit_selection_transaction:
        Option<WaveformEditSelectionSnapshot>,
    pub(in crate::native_app) pending_play_selection_retarget: bool,
    pub(in crate::native_app) pending_play_selection_retarget_cycle:
        Option<PendingPlaySelectionRetargetCycle>,
}

impl WaveformAppState {
    pub(in crate::native_app) fn new(current: WaveformState) -> Self {
        Self {
            current,
            display: WaveformDisplayState::Authoritative,
            starmap_drag_restore: None,
            load: WaveformLoadState::default(),
            cache: WaveformCacheState::default(),
            pending_play_selection_transaction: None,
            pending_edit_fade_transaction: None,
            pending_edit_selection_transaction: None,
            pending_play_selection_retarget: false,
            pending_play_selection_retarget_cycle: None,
        }
    }

    pub(in crate::native_app) fn mark_current_authoritative(&mut self) {
        self.display = WaveformDisplayState::Authoritative;
        self.starmap_drag_restore = None;
    }

    pub(in crate::native_app) fn capture_starmap_drag_restore(&mut self) {
        if self.starmap_drag_restore.is_some()
            || self.instant_preview_active()
            || !self.current.has_loaded_sample()
        {
            return;
        }
        let mut snapshot = self.current.clone();
        snapshot.stop_playback();
        self.starmap_drag_restore = Some(snapshot);
    }

    pub(in crate::native_app) fn restore_starmap_drag_snapshot(&mut self) -> Option<WaveformState> {
        let Some(snapshot) = self.starmap_drag_restore.take() else {
            return None;
        };
        if !self.instant_preview_active() {
            return None;
        }
        self.display = WaveformDisplayState::Authoritative;
        Some(std::mem::replace(&mut self.current, snapshot))
    }

    pub(in crate::native_app) fn replace_current_with_instant_waveform_preview(
        &mut self,
        preview: InstantWaveformPreview,
    ) -> WaveformState {
        let path = preview.path().to_path_buf();
        let tier = preview.tier;
        let previous = std::mem::replace(
            &mut self.current,
            WaveformState::from_cached_file(preview.file),
        );
        self.display = WaveformDisplayState::InstantPreview { path, tier };
        previous
    }

    pub(in crate::native_app) fn replace_current_with_instant_waveform_preview_loading(
        &mut self,
        path: PathBuf,
    ) -> WaveformState {
        let previous = std::mem::replace(&mut self.current, WaveformState::empty());
        self.display = WaveformDisplayState::InstantPreviewLoading { path };
        previous
    }

    pub(in crate::native_app) fn instant_preview_active(&self) -> bool {
        matches!(
            self.display,
            WaveformDisplayState::InstantPreview { .. }
                | WaveformDisplayState::InstantPreviewLoading { .. }
        )
    }

    pub(in crate::native_app) fn instant_preview_tier(&self) -> Option<InstantWaveformPreviewTier> {
        match self.display {
            WaveformDisplayState::InstantPreview { tier, .. } => Some(tier),
            WaveformDisplayState::Authoritative
            | WaveformDisplayState::InstantPreviewLoading { .. } => None,
        }
    }

    pub(in crate::native_app) fn instant_preview_path(&self) -> Option<&Path> {
        match &self.display {
            WaveformDisplayState::InstantPreview { path, .. }
            | WaveformDisplayState::InstantPreviewLoading { path } => Some(path.as_path()),
            WaveformDisplayState::Authoritative => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_app) enum WaveformDisplayState {
    Authoritative,
    InstantPreview {
        path: PathBuf,
        tier: InstantWaveformPreviewTier,
    },
    InstantPreviewLoading {
        path: PathBuf,
    },
}
