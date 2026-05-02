use super::{NativeRunOptions, NativeRunReport, NativeRuntimeArtifacts, WindowIconRgba};
use crate::app::{
    controller::ui::hotkeys::{self, KeyPress},
    state::FocusContext,
};
use crate::app_core::actions::{
    NativeAppBridge, NativeAppModel, NativeFrameBuildResult, NativeGuiAutomationSnapshot,
    NativeMotionModel, NativeUiAction,
};
use std::sync::Arc;

/// Converts app-level Vello launch options into the hosted `radiant` representation.
///
/// Mapping is intentionally field-for-field to preserve behavior and avoid
/// hidden launch-time mutations.
impl From<NativeRunOptions> for radiant::compat::legacy_shell::NativeRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            title: value.title,
            inner_size: value.inner_size,
            min_inner_size: value.min_inner_size,
            maximized: value.maximized,
            decorations: value.decorations,
            icon: value.icon.map(Into::into),
            target_fps: value.target_fps,
        }
    }
}

/// Converts app-level icon payloads into `radiant` host icon payloads.
///
/// All pixel bytes are forwarded unchanged; callers remain responsible for
/// supplying valid RGBA data and matching dimensions.
impl From<WindowIconRgba> for radiant::compat::legacy_shell::WindowIconRgba {
    fn from(value: WindowIconRgba) -> Self {
        Self {
            rgba: value.rgba,
            width: value.width,
            height: value.height,
        }
    }
}

struct CompatNativeAppBridge<B> {
    inner: B,
}

impl<B> CompatNativeAppBridge<B> {
    fn new(inner: B) -> Self {
        Self { inner }
    }
}

impl<B: NativeAppBridge> radiant::compat::legacy_shell::NativeAppBridge
    for CompatNativeAppBridge<B>
{
    fn project_model(&mut self) -> Arc<radiant::compat::legacy_shell::AppModel> {
        let model = self.inner.project_model();
        Arc::new(model.as_ref().into())
    }

    fn pull_model(&mut self) -> radiant::compat::legacy_shell::AppModel {
        self.inner.pull_model().into()
    }

    fn pull_model_arc(&mut self) -> Arc<radiant::compat::legacy_shell::AppModel> {
        let model = self.inner.pull_model_arc();
        Arc::new(model.as_ref().into())
    }

    fn project_motion_model(&mut self) -> Option<radiant::compat::legacy_shell::NativeMotionModel> {
        self.inner
            .project_motion_model()
            .map(NativeMotionModel::into)
    }

    fn take_dirty_segments(&mut self) -> radiant::compat::legacy_shell::DirtySegments {
        self.inner.take_dirty_segments().into()
    }

    fn take_segment_revisions(&mut self) -> radiant::compat::legacy_shell::SegmentRevisions {
        self.inner.take_segment_revisions().into()
    }

    fn resolve_hotkey_press(
        &mut self,
        pending_chord: Option<radiant::compat::legacy_shell::KeyPress>,
        press: radiant::compat::legacy_shell::KeyPress,
        focus: radiant::compat::legacy_shell::FocusContextModel,
    ) -> radiant::compat::legacy_shell::HotkeyResolution {
        let resolution = hotkeys::resolve_hotkey_press(
            pending_chord.map(keypress_from_radiant),
            keypress_from_radiant(press),
            focus_context_from_radiant(focus),
        );
        radiant::compat::legacy_shell::HotkeyResolution {
            action: resolution.action.map(Into::into),
            handled: resolution.handled,
            pending_chord: resolution.pending_chord.map(keypress_to_radiant),
        }
    }

    fn reduce_action(&mut self, action: radiant::compat::legacy_shell::UiAction) {
        self.inner.reduce_action(NativeUiAction::from(action));
    }

    fn take_last_action_handled(&mut self) -> Option<bool> {
        self.inner.take_last_action_handled()
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.inner.install_repaint_signal(signal);
    }

    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, hwnd: isize) {
        self.inner.set_external_drag_hwnd(hwnd);
    }

    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) -> bool {
        self.inner
            .maybe_launch_external_drag(pointer_outside, pointer_left)
    }

    fn observe_frame_result(&mut self, result: radiant::compat::legacy_shell::FrameBuildResult) {
        self.inner
            .observe_frame_result(NativeFrameBuildResult::from(result));
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.inner
            .on_runtime_exit()
            .and_then(|artifact| serde_json::to_value(artifact).ok())
    }
}

fn native_run_report_from_radiant(
    report: radiant::compat::legacy_shell::NativeRunReport,
) -> NativeRunReport {
    NativeRunReport {
        artifacts: NativeRuntimeArtifacts {
            startup_timing: report.artifacts.startup_timing,
            shutdown_timing: report
                .artifacts
                .shutdown_timing
                .and_then(|value| serde_json::from_value(value).ok()),
        },
        result: report.result,
    }
}

fn focus_context_from_radiant(
    focus: radiant::compat::legacy_shell::FocusContextModel,
) -> FocusContext {
    match focus {
        radiant::compat::legacy_shell::FocusContextModel::None => FocusContext::None,
        radiant::compat::legacy_shell::FocusContextModel::Timeline => FocusContext::Waveform,
        radiant::compat::legacy_shell::FocusContextModel::ContentList => {
            FocusContext::SampleBrowser
        }
        radiant::compat::legacy_shell::FocusContextModel::NavigationTree => {
            FocusContext::SourceFolders
        }
        radiant::compat::legacy_shell::FocusContextModel::NavigationList => {
            FocusContext::SourcesList
        }
    }
}

fn keypress_from_radiant(press: radiant::compat::legacy_shell::KeyPress) -> KeyPress {
    KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

fn keypress_to_radiant(press: KeyPress) -> radiant::compat::legacy_shell::KeyPress {
    radiant::compat::legacy_shell::KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

pub(super) fn run_native_vello_app<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    radiant::compat::legacy_shell::run_native_vello_app(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    )
}

pub(super) fn run_native_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    let report = radiant::compat::legacy_shell::run_native_vello_app_with_artifacts(
        options.into(),
        CompatNativeAppBridge::new(bridge),
    );
    native_run_report_from_radiant(report)
}

pub(super) fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let compat_model = radiant::compat::legacy_shell::AppModel::from(model);
    radiant::compat::legacy_shell::capture_gui_automation_snapshot(viewport, &compat_model).into()
}

#[cfg(test)]
pub(super) fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> impl serde::Serialize {
    let compat_model = radiant::compat::legacy_shell::AppModel::from(model);
    radiant::compat::legacy_shell::capture_native_shell_shot_snapshot(name, viewport, &compat_model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_run_options_map_field_for_field_to_radiant_compat_options() {
        let options = NativeRunOptions {
            title: String::from("Sempal test host"),
            inner_size: Some([1280.0, 720.0]),
            min_inner_size: Some([640.0, 360.0]),
            maximized: true,
            decorations: false,
            icon: Some(WindowIconRgba {
                rgba: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
            }),
            target_fps: 90,
        };

        let compat: radiant::compat::legacy_shell::NativeRunOptions = options.into();

        assert_eq!(compat.title, "Sempal test host");
        assert_eq!(compat.inner_size, Some([1280.0, 720.0]));
        assert_eq!(compat.min_inner_size, Some([640.0, 360.0]));
        assert!(compat.maximized);
        assert!(!compat.decorations);
        assert_eq!(compat.target_fps, 90);
        let icon = compat.icon.expect("icon should be forwarded");
        assert_eq!(icon.rgba, vec![255, 0, 0, 255]);
        assert_eq!(icon.width, 1);
        assert_eq!(icon.height, 1);
    }
}
