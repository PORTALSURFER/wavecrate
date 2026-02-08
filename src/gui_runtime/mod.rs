//! Shared GUI runtime host implementations re-exported from `radiant`.

/// RGBA icon payload used by native runtime hosts.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}
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

impl From<NativeRunOptions> for radiant::gui_runtime::NativeRunOptions {
    fn from(value: NativeRunOptions) -> Self {
        Self {
            title: value.title,
            inner_size: value.inner_size,
            min_inner_size: value.min_inner_size,
            maximized: value.maximized,
            icon: value.icon.map(Into::into),
        }
    }
}

impl From<WindowIconRgba> for radiant::gui_runtime::WindowIconRgba {
    fn from(value: WindowIconRgba) -> Self {
        Self {
            rgba: value.rgba,
            width: value.width,
            height: value.height,
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
