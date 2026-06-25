use radiant::runtime::{
    NativeFrameOptions, NativeRunOptions, NativeTextOptions, NativeWindowBehavior,
    NativeWindowGeometry, NativeWindowOptions,
};
use wavecrate::native_runtime::wavecrate_ui_font_path;

use super::icon::wavecrate_window_icon;

const APP_NAME: &str = "Wavecrate";
const RELEASE_CHANNEL: &str = "Alpha";

pub(in crate::native_app) fn default_window_title() -> String {
    format!(
        "{APP_NAME} {} b{} - {RELEASE_CHANNEL}",
        env!("CARGO_PKG_VERSION"),
        env!("WAVECRATE_BUILD_NUMBER")
    )
}

pub(super) fn native_run_options(debug_layout: bool) -> NativeRunOptions {
    NativeRunOptions {
        window: NativeWindowOptions {
            title: default_window_title(),
            geometry: NativeWindowGeometry {
                inner_size: Some([960.0, 540.0]),
                min_inner_size: Some([640.0, 360.0]),
                ..NativeWindowGeometry::default()
            },
            behavior: NativeWindowBehavior {
                drag_and_drop: true,
                maximized: true,
                ..NativeWindowBehavior::default()
            },
            icon: wavecrate_window_icon(),
            ..NativeWindowOptions::default()
        },
        frame: NativeFrameOptions {
            debug_layout,
            ..NativeFrameOptions::default()
        },
        text: NativeTextOptions {
            embedded_fonts: Vec::new(),
            font_paths: vec![wavecrate_ui_font_path()],
        },
        ..NativeRunOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_native_run_options_start_main_window_maximized() {
        let options = native_run_options(false);

        assert!(options.window.behavior.maximized);
        assert!(options.window.behavior.decorations);
        assert_eq!(options.window.title, default_window_title());
    }

    #[test]
    fn debug_layout_arg_does_not_change_default_window_mode() {
        let options = native_run_options(true);

        assert!(options.frame.debug_layout);
        assert!(options.window.behavior.maximized);
        assert!(options.window.behavior.decorations);
    }

    #[test]
    fn default_native_run_options_include_wavecrate_icon() {
        let options = native_run_options(false);
        let icon = options.window.icon.expect("Wavecrate icon should decode");

        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(
            icon.rgba.len(),
            icon.width as usize * icon.height as usize * 4
        );
        assert!(icon.rgba.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn bundled_wavecrate_icon_decodes_to_runtime_payload() {
        let icon =
            crate::native_app::shell::launch::icon::decode_bundled_wavecrate_window_icon_for_tests(
            )
            .expect("Wavecrate icon should decode");

        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }
}
