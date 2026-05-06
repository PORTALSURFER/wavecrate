use super::*;

/// Converts app-level Vello launch options into the generic `radiant` runtime representation.
///
/// Mapping is intentionally field-for-field to preserve behavior and avoid
/// hidden launch-time mutations.
impl From<NativeRunOptions> for radiant::gui_runtime::NativeRunOptions {
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
impl From<WindowIconRgba> for radiant::gui_runtime::WindowIconRgba {
    fn from(value: WindowIconRgba) -> Self {
        Self {
            rgba: value.rgba,
            width: value.width,
            height: value.height,
        }
    }
}

/// Run Sempal through the generic Radiant Vello runtime.
pub(crate) fn run_native_vello_app<B: NativeAppBridge + 'static>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    run_native_vello_app_with_artifacts(options, bridge).result
}

/// Run Sempal through the generic Radiant Vello runtime and return launch artifacts.
pub(crate) fn run_native_vello_app_with_artifacts<B: NativeAppBridge + 'static>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    let report = radiant::gui_runtime::run_native_vello_runtime_with_artifacts(
        options.into(),
        SempalRuntimeBridge::new(bridge),
    );
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
