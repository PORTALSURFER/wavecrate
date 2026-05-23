use super::*;
use crate::gui::repaint::RepaintSignal;
use std::sync::atomic::Ordering;

pub(super) struct RecordingBridge {
    pub(super) model: Arc<NativeAppModel>,
    pub(super) reduced: Vec<UiAction>,
    pub(super) repaint_installed: Arc<AtomicBool>,
    pub(super) exit_status: Option<String>,
}

impl NativeAppBridge for RecordingBridge {
    fn project_model(&mut self) -> Arc<NativeAppModel> {
        Arc::clone(&self.model)
    }

    fn reduce_action(&mut self, action: UiAction) {
        match &action {
            UiAction::SetBrowserSearch { query } => {
                Arc::make_mut(&mut self.model).browser.search_query = query.clone();
            }
            UiAction::SetBrowserTagSidebarInput { value } => {
                Arc::make_mut(&mut self.model)
                    .browser
                    .tag_sidebar
                    .input_value = value.clone();
            }
            UiAction::CommitBrowserTagSidebarInput => {
                Arc::make_mut(&mut self.model)
                    .browser
                    .tag_sidebar
                    .input_value
                    .clear();
            }
            _ => {}
        }
        self.reduced.push(action);
    }

    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {
        self.repaint_installed.store(true, Ordering::Release);
    }

    fn on_runtime_exit(&mut self) -> Option<crate::gui_runtime::NativeShutdownTimingArtifact> {
        Some(crate::gui_runtime::NativeShutdownTimingArtifact {
            status: self.exit_status.take()?,
            failure_reason: None,
            bridge_exit_flush_ms: None,
            config_persist_ms: None,
            controller_jobs_shutdown_ms: None,
            analysis_shutdown_ms: None,
            controller_shutdown_ms: None,
            runtime_exit_total_ms: None,
        })
    }
}

pub(super) struct MotionOnlyRecordingBridge {
    pub(super) model: Arc<NativeAppModel>,
    pub(super) motion_model: Option<NativeMotionModel>,
    pub(super) model_pull_count: usize,
    pub(super) motion_pull_count: usize,
}

impl NativeAppBridge for MotionOnlyRecordingBridge {
    fn project_model(&mut self) -> Arc<NativeAppModel> {
        self.model_pull_count += 1;
        Arc::clone(&self.model)
    }

    fn pull_model_arc(&mut self) -> Arc<NativeAppModel> {
        self.project_model()
    }

    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.motion_pull_count += 1;
        self.motion_model.clone()
    }
}

pub(super) struct TestRepaintSignal;

impl RepaintSignal for TestRepaintSignal {
    fn request_repaint(&self) {}
}

#[derive(Default)]
pub(super) struct NativeDropRecordingBridge {
    pub(super) events: Vec<NativeFileDropEvent>,
}

impl NativeAppBridge for NativeDropRecordingBridge {
    fn project_model(&mut self) -> Arc<NativeAppModel> {
        Arc::new(NativeAppModel::default())
    }

    fn handle_native_file_drop(&mut self, event: NativeFileDropEvent) {
        self.events.push(event);
    }
}

/// Return the retained shell descriptor projected by the bridge surface.
pub(super) fn retained_shell_descriptor(
    bridge: &mut WavecrateRuntimeBridge<RecordingBridge>,
) -> RetainedSurfaceDescriptor {
    let surface = bridge.project_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            radiant::gui::types::Vector2::new(1280.0, 720.0),
        ),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::CustomSurface(surface) => surface.retained,
            _ => None,
        })
        .expect("generic bridge should project retained shell metadata")
}

/// Return the retained shell descriptor projected by a motion-only bridge.
pub(super) fn retained_motion_descriptor(
    bridge: &mut WavecrateRuntimeBridge<MotionOnlyRecordingBridge>,
) -> RetainedSurfaceDescriptor {
    let surface = bridge.project_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        radiant::gui::types::Rect::from_min_size(
            radiant::gui::types::Point::new(0.0, 0.0),
            radiant::gui::types::Vector2::new(1280.0, 720.0),
        ),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::CustomSurface(surface) => surface.retained,
            _ => None,
        })
        .expect("generic bridge should project retained shell metadata")
}

/// Return whether a frame contains the narrow waveform playhead marker.
pub(super) fn frame_contains_playhead_marker(
    frame: &PaintFrame,
    layout: &ShellLayout,
    style: &StyleTokens,
) -> bool {
    frame.primitives.iter().any(|primitive| match primitive {
        crate::gui::paint::Primitive::Rect(rect) => {
            rect.color == style.accent_copper
                && rect.rect.min.x >= layout.waveform_plot.min.x
                && rect.rect.max.x <= layout.waveform_plot.max.x
                && rect.rect.min.y >= layout.waveform_plot.min.y
                && rect.rect.max.y <= layout.waveform_plot.max.y
                && rect.rect.width() <= (style.sizing.border_width * 2.0).max(2.0)
        }
        _ => false,
    })
}
