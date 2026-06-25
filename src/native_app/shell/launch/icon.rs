use image::ImageFormat;
use radiant::runtime::WindowIconRgba;

const APP_ICON_ICO: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/logo3.ico"));

pub(super) fn wavecrate_window_icon() -> Option<WindowIconRgba> {
    match decode_bundled_wavecrate_window_icon() {
        Ok(icon) => Some(icon),
        Err(error) => {
            tracing::warn!(%error, "failed to decode bundled Wavecrate window icon");
            None
        }
    }
}

fn decode_bundled_wavecrate_window_icon() -> Result<WindowIconRgba, String> {
    let image = image::load_from_memory_with_format(APP_ICON_ICO, ImageFormat::Ico)
        .map_err(|error| format!("decode logo3.ico: {error}"))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(WindowIconRgba {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

#[cfg(test)]
pub(super) fn decode_bundled_wavecrate_window_icon_for_tests() -> Result<WindowIconRgba, String> {
    decode_bundled_wavecrate_window_icon()
}
