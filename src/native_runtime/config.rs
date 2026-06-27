use std::path::PathBuf;

/// Bundled Wavecrate UI font asset.
pub const WAVECRATE_UI_FONT_ASSET: &str = "assets/fonts/IoskeleyMono-Regular.ttf";
/// Bundled Wavecrate UI font bytes used by release builds.
pub const WAVECRATE_UI_FONT_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/IoskeleyMono-Regular.ttf"
));

/// Return the bundled Wavecrate UI font path used before native font fallbacks.
pub fn wavecrate_ui_font_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WAVECRATE_UI_FONT_ASSET)
}

/// RGBA icon payload used by UI runtime hosts.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// UI runtime launch options for Vello hosts.
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
    /// Whether native window decorations remain enabled.
    pub decorations: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
    /// Target frame rate for animation-driven redraws.
    pub target_fps: u32,
    /// Paint red layout-boundary strokes over every projected layout element.
    pub debug_layout: bool,
}
