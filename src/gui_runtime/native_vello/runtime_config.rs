use super::*;

/// High-refresh surface present-mode preference order for animation-heavy playback UI.
const HIGH_REFRESH_PRESENT_MODE_CANDIDATES: [wgpu::PresentMode; 3] = [
    wgpu::PresentMode::Mailbox,
    wgpu::PresentMode::Immediate,
    wgpu::PresentMode::AutoVsync,
];
/// Standard present-mode preference order for non-high-refresh UI.
const STANDARD_PRESENT_MODE_CANDIDATES: [wgpu::PresentMode; 1] = [wgpu::PresentMode::AutoVsync];

/// Return the ordered present-mode fallback chain for the configured frame target.
pub(in crate::gui_runtime::native_vello) fn present_mode_candidates(
    target_fps: u32,
) -> &'static [wgpu::PresentMode] {
    if target_fps >= 120 {
        &HIGH_REFRESH_PRESENT_MODE_CANDIDATES
    } else {
        &STANDARD_PRESENT_MODE_CANDIDATES
    }
}

pub(in crate::gui_runtime::native_vello) fn select_present_mode(
    target_fps: u32,
    supported_present_modes: &[wgpu::PresentMode],
) -> wgpu::PresentMode {
    present_mode_candidates(target_fps)
        .iter()
        .copied()
        .find(|mode| present_mode_is_supported(*mode, supported_present_modes))
        .or_else(|| supported_present_modes.first().copied())
        .unwrap_or(wgpu::PresentMode::Fifo)
}

fn present_mode_is_supported(
    present_mode: wgpu::PresentMode,
    supported_present_modes: &[wgpu::PresentMode],
) -> bool {
    matches!(
        present_mode,
        wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync
    ) || supported_present_modes.contains(&present_mode)
}

/// Build renderer startup options for the native shell's fixed AA strategy.
///
/// The native runtime currently renders every frame with [`AaConfig::Area`], so
/// startup should avoid compiling MSAA shader variants that will never be used.
pub(in crate::gui_runtime::native_vello) fn startup_renderer_options() -> RendererOptions {
    RendererOptions {
        antialiasing_support: AaSupport::area_only(),
        ..RendererOptions::default()
    }
}
