use radiant::runtime::{
    NativeFrameOptions, NativeRunOptions, NativeTextOptions, NativeWindowBehavior,
    NativeWindowGeometry, NativeWindowOptions,
};
use wavecrate::native_runtime::wavecrate_ui_font_path;

pub(in crate::native_app) const DEFAULT_WINDOW_TITLE: &str = "Wavecrate - alpha";

pub(super) fn native_run_options(debug_layout: bool) -> NativeRunOptions {
    NativeRunOptions {
        window: NativeWindowOptions {
            title: String::from(DEFAULT_WINDOW_TITLE),
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
        assert_eq!(options.window.title, DEFAULT_WINDOW_TITLE);
    }

    #[test]
    fn debug_layout_arg_does_not_change_default_window_mode() {
        let options = native_run_options(true);

        assert!(options.frame.debug_layout);
        assert!(options.window.behavior.maximized);
        assert!(options.window.behavior.decorations);
    }
}
