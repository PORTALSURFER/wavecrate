//! Shared waveform-image translation helpers for native-shell projection paths.

use crate::gui::types::ImageRgba;
use crate::waveform::WaveformImage;
use std::sync::Arc;

/// Convert a rendered waveform image into the native immutable RGBA payload.
///
/// This helper is the authoritative bridge between controller-owned waveform
/// rasters and the native-shell image payload consumed by retained projection
/// caches. Callers should reuse this instead of duplicating byte packing logic.
pub(crate) fn waveform_image_to_native_rgba(image: &WaveformImage) -> Option<Arc<ImageRgba>> {
    if image.size[0] == 0 || image.size[1] == 0 {
        return None;
    }
    let mut pixels = Vec::with_capacity(
        image.size[0]
            .saturating_mul(image.size[1])
            .saturating_mul(4),
    );
    for pixel in &image.pixels {
        pixels.push(pixel.r());
        pixels.push(pixel.g());
        pixels.push(pixel.b());
        pixels.push(pixel.a());
    }
    ImageRgba::new(image.size[0], image.size[1], pixels).map(Arc::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform::{WaveformImage, WaveformRgba};

    #[test]
    fn waveform_image_to_native_rgba_packs_pixels_in_rgba_order() {
        let image = WaveformImage::new(
            [2, 1],
            vec![
                WaveformRgba::from_rgba_unmultiplied(1, 2, 3, 4),
                WaveformRgba::from_rgba_unmultiplied(5, 6, 7, 8),
            ],
        );

        let native = waveform_image_to_native_rgba(&image).expect("expected translated image");

        assert_eq!(native.width, 2);
        assert_eq!(native.height, 1);
        assert_eq!(native.pixels.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn waveform_image_to_native_rgba_rejects_zero_sized_images() {
        let image = WaveformImage::new([0, 0], Vec::new());

        assert!(waveform_image_to_native_rgba(&image).is_none());
    }
}
