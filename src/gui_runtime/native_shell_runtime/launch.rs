use super::*;

/// Converts app-level Vello launch options into the generic `radiant` runtime representation.
///
/// Mapping is intentionally field-for-field to preserve behavior and avoid
/// hidden launch-time mutations.
impl From<NativeRunOptions> for radiant::gui_runtime::NativeRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            window: radiant::gui_runtime::NativeWindowOptions {
                title: value.title,
                geometry: radiant::gui_runtime::NativeWindowGeometry {
                    inner_size: value.inner_size,
                    position: None,
                    min_inner_size: value.min_inner_size,
                },
                behavior: radiant::gui_runtime::NativeWindowBehavior {
                    maximized: value.maximized,
                    decorations: value.decorations,
                    drag_and_drop: true,
                    owner_window_handle: None,
                    skip_taskbar: false,
                    mode: radiant::gui_runtime::NativeWindowMode::default(),
                },
                icon: value.icon.map(Into::into),
            },
            frame: radiant::gui_runtime::NativeFrameOptions {
                target_fps: value.target_fps,
                debug_layout: value.debug_layout,
                retained_surface_cache: radiant::runtime::RetainedSurfaceCachePolicy::default(),
            },
            gpu: radiant::gui_runtime::NativeGpuOptions::default(),
            text: radiant::gui_runtime::NativeTextOptions {
                embedded_fonts: Vec::new(),
                font_paths: vec![crate::gui_runtime::wavecrate_ui_font_path()],
            },
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

/// Run Wavecrate through the generic Radiant Vello runtime.
pub(crate) fn run_native_vello_app<B: NativeAppBridge + 'static>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    run_native_vello_app_with_artifacts(options, bridge).result
}

/// Run Wavecrate through the generic Radiant Vello runtime and return launch artifacts.
pub(crate) fn run_native_vello_app_with_artifacts<B: NativeAppBridge + 'static>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    let title = options.title.clone();
    let report = radiant::window(title)
        .options(options.into())
        .run_bridge_with_artifacts(WavecrateRuntimeBridge::new(bridge));
    NativeRunReport {
        artifacts: NativeRuntimeArtifacts {
            startup_timing: report.artifacts.startup_timing,
            shutdown_timing: report
                .artifacts
                .shutdown_timing
                .and_then(|value| serde_json::from_value(value).ok()),
        },
        result: report.result.map_err(|err| err.to_string()),
    }
}
