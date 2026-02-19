//! Icon loading helpers for the native bootstrap path.

use sempal::gui_runtime::WindowIconRgba;

/// Loads the application icon, preferring the ICO asset and falling back to PNG.
pub fn load_app_icon() -> Option<WindowIconRgba> {
    decode_icon(include_bytes!("../assets/logo3.ico")).or_else(|| {
        eprintln!("Failed to decode logo3.ico; falling back to PNG icon.");
        let fallback = decode_icon(include_bytes!("../assets/logo3.png"));
        if fallback.is_none() {
            eprintln!("Failed to decode logo3.png fallback for window icon.");
        }
        fallback
    })
}

fn decode_icon(bytes: &[u8]) -> Option<WindowIconRgba> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(WindowIconRgba {
        rgba: image.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icons_decode() {
        assert!(decode_icon(include_bytes!("../assets/logo3.ico")).is_some());
        assert!(decode_icon(include_bytes!("../assets/logo3.png")).is_some());
    }
}
