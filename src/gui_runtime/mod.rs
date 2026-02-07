//! Shared GUI runtime host implementations re-exported from `radiant`.

/// RGBA icon payload used by native runtime hosts.
pub type WindowIconRgba = radiant::gui_runtime::WindowIconRgba;
/// Native runtime launch options for Vello hosts.
#[derive(Clone, Debug, Default)]
pub struct NativeRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
}

impl From<NativeRunOptions> for radiant::gui_runtime::EguiRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            title: value.title,
            inner_size: value.inner_size,
            min_inner_size: value.min_inner_size,
            maximized: value.maximized,
            icon: value.icon,
        }
    }
}

/// Run the native Vello backend with a host-provided app bridge.
pub fn run_native_vello_app<B: radiant::app::NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> Result<(), String> {
    radiant::gui_runtime::run_native_vello_app(options.into(), bridge)
}

/// Run the native Vello backend preview shell for backend smoke-testing.
pub fn run_native_vello_preview(options: NativeRunOptions) -> Result<(), String> {
    radiant::gui_runtime::run_native_vello_preview(options.into())
}
