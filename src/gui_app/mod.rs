//! Transitional GUI app exports used while migrating away from `egui`.

mod bridge;

use crate::{audio::AudioPlayer, egui_app::ui::EguiApp, waveform::WaveformRenderer};

/// Default viewport size for the main application window.
pub use crate::egui_app::ui::DEFAULT_VIEWPORT_SIZE;
/// Minimum viewport size for the main application window.
pub use crate::egui_app::ui::MIN_VIEWPORT_SIZE;
/// Native runtime bridge for the `radiant` backend path.
pub use bridge::SempalNativeBridge;
/// Construct a native runtime bridge for the `radiant` backend path.
pub use bridge::new_native_bridge;

/// Current app implementation used by the GUI entrypoint.
pub type SempalGuiApp = EguiApp;

/// Construct the current GUI app implementation.
pub fn new_sempal_app(
    renderer: WaveformRenderer,
    player: Option<std::rc::Rc<std::cell::RefCell<AudioPlayer>>>,
) -> Result<SempalGuiApp, String> {
    SempalGuiApp::new(renderer, player)
}
