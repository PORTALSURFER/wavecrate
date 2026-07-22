use radiant::runtime::{
    EmbeddedFont, NativeFrameOptions, NativeRunOptions, NativeTextOptions, NativeWindowBehavior,
    NativeWindowGeometry, NativeWindowOptions,
};
use wavecrate::native_runtime::{WAVECRATE_UI_FONT_BYTES, wavecrate_ui_font_path};

use super::icon::wavecrate_window_icon;

const APP_NAME: &str = "Wavecrate";
const PERF_GUARD_STARTUP_HIDDEN_ENV: &str = "WAVECRATE_PERF_GUARD_STARTUP_HIDDEN";

pub(in crate::native_app) fn default_window_title() -> String {
    let metadata = wavecrate::release_metadata::CURRENT;
    format!(
        "{APP_NAME} {} b{} - {}",
        metadata.version,
        metadata.build_number,
        metadata.release_channel().display_label()
    )
}

pub(super) fn native_run_options(debug_layout: bool) -> NativeRunOptions {
    native_run_options_with_startup_hidden(debug_layout, perf_guard_startup_hidden())
}

fn native_run_options_with_startup_hidden(
    debug_layout: bool,
    startup_hidden: bool,
) -> NativeRunOptions {
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
                integrated_titlebar: true,
                integrated_titlebar_drag_region_height: Some(38.0),
                maximized: true,
                reveal_after_surface_setup: !startup_hidden,
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
            embedded_fonts: vec![EmbeddedFont::from_static(WAVECRATE_UI_FONT_BYTES)],
            font_paths: vec![wavecrate_ui_font_path()],
        },
        ..NativeRunOptions::default()
    }
}

fn perf_guard_startup_hidden() -> bool {
    std::env::var(PERF_GUARD_STARTUP_HIDDEN_ENV).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_native_run_options_start_main_window_maximized() {
        let options = native_run_options(false);

        assert!(options.window.behavior.maximized);
        assert!(options.window.behavior.decorations);
        assert!(options.window.behavior.integrated_titlebar);
        assert_eq!(options.window.title, default_window_title());
    }

    #[test]
    fn debug_layout_arg_does_not_change_default_window_mode() {
        let options = native_run_options(true);

        assert!(options.frame.debug_layout);
        assert!(options.window.behavior.maximized);
        assert!(options.window.behavior.decorations);
        assert!(options.window.behavior.reveal_after_surface_setup);
    }

    #[test]
    fn perf_guard_startup_hidden_keeps_main_window_unrevealed() {
        let options = native_run_options_with_startup_hidden(false, true);

        assert!(!options.window.behavior.reveal_after_surface_setup);
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
    fn default_native_run_options_embed_wavecrate_font() {
        let options = native_run_options(false);

        assert_eq!(options.text.embedded_fonts.len(), 1);
        assert_eq!(
            options.text.embedded_fonts[0].bytes(),
            WAVECRATE_UI_FONT_BYTES
        );
        assert_eq!(options.text.font_paths, vec![wavecrate_ui_font_path()]);
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
