use std::time::Duration;

use radiant::runtime::{RepaintScope, SurfaceRevisions};

use super::super::diagnostics::PlayheadFrameMessageDiagnostics;
use crate::native_app::{
    app::{NativeAppState, SamplePlaybackSession},
    starmap_audition_telemetry::{
        self as starmap_telemetry, StarmapAuditionCounter, StarmapAuditionDuration,
    },
};

#[derive(Default)]
/// Converts frame-relevant application state into monotonic runtime revision keys.
pub(in crate::native_app) struct FrameSurfaceRevisionTracker {
    last: Option<FrameSurfaceRevisionInputs>,
    revisions: SurfaceRevisions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameSurfaceRevisionInputs {
    structure: FrameStructureState,
    layout: FrameLayoutState,
    projection: FrameProjectionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameStructureState {
    playing: bool,
    drag_hover_auto_expand_pending: bool,
    folder_progress_active: bool,
    normalization_progress_active: bool,
    file_move_progress_active: bool,
    source_cache_progress_active: bool,
    waveform_loading_active: bool,
    sample_loading: bool,
    audio_opening: bool,
    audio_settings_error_active: bool,
    startup_source_scan_pending: bool,
    startup_auto_load_pending: bool,
    pending_playback_start: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameLayoutState {
    audio_output_sample_rate: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameProjectionState {
    playback_visual_generation: u64,
    play_selection_flash_active: bool,
    copy_flash_frames: u8,
    protected_source_error_flash_frames: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameSurfaceRevisionSample {
    revisions: SurfaceRevisions,
    scope: RepaintScope,
    playback_involved: bool,
}

impl NativeAppState {
    pub(in crate::native_app) fn frame_surface_revisions(&mut self) -> SurfaceRevisions {
        let inputs = FrameSurfaceRevisionInputs::from_state(self);
        let sample = self.frame_surface_revision_tracker.observe(inputs);
        if sample.playback_involved {
            self.playhead_frame_diagnostics
                .record_frame_message(PlayheadFrameMessageDiagnostics {
                    paint_only: sample.scope.is_paint_only(),
                    reason: frame_repaint_reason(sample.scope),
                });
        }
        sample.revisions
    }

    #[cfg(test)]
    pub(in crate::native_app) fn capture_frame_surface_revisions(&mut self) -> SurfaceRevisions {
        self.frame_surface_revisions()
    }

    #[cfg(test)]
    pub(in crate::native_app) fn frame_can_use_paint_only_since(
        &mut self,
        before: SurfaceRevisions,
    ) -> bool {
        self.frame_scope_since(before).is_paint_only()
    }

    #[cfg(test)]
    pub(in crate::native_app) fn frame_scope_since(
        &mut self,
        before: SurfaceRevisions,
    ) -> RepaintScope {
        self.frame_surface_revisions().repaint_scope_since(before)
    }

    pub(in crate::native_app) fn observe_playhead_native_frame_diagnostics(
        &mut self,
        diagnostics: radiant::runtime::NativeFrameDiagnostics,
    ) {
        self.playhead_frame_diagnostics
            .observe_native_frame(diagnostics);
    }
}

pub(super) fn log_runtime_playback_event(
    stage: &'static str,
    outcome: &'static str,
    starmap_counter: Option<StarmapAuditionCounter>,
    session: &SamplePlaybackSession,
    elapsed: Option<Duration>,
    error: Option<&str>,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "wavecrate_native_playback",
        stage,
        outcome,
        origin = session.request.origin,
        source_kind = session.source_kind,
        path = %session.request.path,
        start_ratio = session.request.span.0,
        end_ratio = session.request.span.1,
        elapsed_ms = elapsed.map(duration_ms).unwrap_or(0.0),
        error = error.unwrap_or_default(),
        "Native playback runtime event"
    );
    if session.request.origin != "starmap_drag" {
        return;
    }
    if let Some(elapsed) = elapsed {
        starmap_telemetry::record_duration(StarmapAuditionDuration::RuntimeStart, elapsed);
    }
    starmap_telemetry::record_event(
        starmap_counter,
        stage,
        outcome,
        Some(session.request.path.as_str()),
        0,
        0,
        true,
        elapsed,
    );
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

impl FrameSurfaceRevisionTracker {
    fn observe(&mut self, current: FrameSurfaceRevisionInputs) -> FrameSurfaceRevisionSample {
        let (scope, playback_involved) = self.last.map_or(
            (RepaintScope::PaintOnly, current.structure.playing),
            |previous| {
                (
                    current.repaint_scope_since(previous),
                    previous.structure.playing || current.structure.playing,
                )
            },
        );
        match scope {
            RepaintScope::Surface => {
                self.revisions.structure = self.revisions.structure.wrapping_add(1);
            }
            RepaintScope::Layout => {
                self.revisions.layout = self.revisions.layout.wrapping_add(1);
            }
            RepaintScope::Projection => {
                self.revisions.projection = self.revisions.projection.wrapping_add(1);
            }
            RepaintScope::PaintOnly => {}
        }
        self.last = Some(current);
        FrameSurfaceRevisionSample {
            revisions: self.revisions,
            scope,
            playback_involved,
        }
    }
}

impl FrameSurfaceRevisionInputs {
    fn from_state(state: &NativeAppState) -> Self {
        Self {
            structure: FrameStructureState {
                playing: state.playback_visual_activity_active(),
                drag_hover_auto_expand_pending: state
                    .library
                    .folder_browser
                    .drag_hover_auto_expand_pending(),
                folder_progress_active: state.library.folder_scan_active(),
                normalization_progress_active: state.background.normalization_progress.is_some(),
                file_move_progress_active: state.background.file_move_progress.is_some(),
                source_cache_progress_active: state
                    .waveform
                    .cache
                    .active_folder_warm_folder_id
                    .is_some(),
                waveform_loading_active: state.waveform.load.label.is_some(),
                sample_loading: state.active_sample_load_task().is_some(),
                audio_opening: state.background.audio_open.active().is_some(),
                audio_settings_error_active: state.audio.settings_error.is_some(),
                startup_source_scan_pending: state.ui.startup.source_scan_pending,
                startup_auto_load_pending: state.ui.startup.auto_load_pending,
                pending_playback_start: state.audio.pending_playback_start.is_some(),
            },
            layout: FrameLayoutState {
                audio_output_sample_rate: state
                    .audio
                    .output_resolved
                    .as_ref()
                    .map(|output| output.sample_rate),
            },
            projection: FrameProjectionState {
                playback_visual_generation: state.waveform.current.playback_visual_generation(),
                play_selection_flash_active: state.waveform.current.play_selection_flash_active(),
                copy_flash_frames: state
                    .library
                    .folder_browser
                    .copy_flash_frames()
                    .max(state.waveform.current.copy_flash_frames()),
                protected_source_error_flash_frames: state
                    .library
                    .folder_browser
                    .protected_source_error_flash_frames()
                    .max(state.waveform.current.protected_source_error_flash_frames()),
            },
        }
    }

    fn repaint_scope_since(self, previous: Self) -> RepaintScope {
        if self.structure != previous.structure {
            RepaintScope::Surface
        } else if self.layout != previous.layout {
            RepaintScope::Layout
        } else if self.projection != previous.projection || previous.requires_projection_refresh() {
            RepaintScope::Projection
        } else {
            RepaintScope::PaintOnly
        }
    }

    fn requires_projection_refresh(self) -> bool {
        self.projection.play_selection_flash_active
            || self.structure.folder_progress_active
            || self.structure.file_move_progress_active
            || self.structure.source_cache_progress_active
            || self.structure.sample_loading
            || self.structure.audio_opening
            || self.structure.startup_source_scan_pending
            || self.structure.startup_auto_load_pending
            || self.structure.pending_playback_start
    }
}

fn frame_repaint_reason(scope: RepaintScope) -> &'static str {
    match scope {
        RepaintScope::Surface => "structure_revision",
        RepaintScope::Layout => "layout_revision",
        RepaintScope::Projection => "projection_revision",
        RepaintScope::PaintOnly => "paint_only",
    }
}
