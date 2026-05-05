use super::*;

fn solid_image(width: usize, height: usize, color: WaveformRgba) -> WaveformImage {
    WaveformImage::new([width, height], vec![color; width * height])
}

#[test]
fn sample_at_frame_uses_linear_interpolation_near_edges() {
    let samples = [0.0_f32, 1.0, 0.0];
    let sample = WaveformRenderer::sample_at_frame(&samples, 1, 0.5, Some(0));
    assert!(
        (sample - 0.5).abs() < 1.0e-6,
        "expected linear edge interpolation, got {sample}"
    );
}

#[test]
fn sample_at_frame_prefers_channel_with_largest_absolute_value() {
    let samples = [0.10_f32, -0.80, 0.20, -0.60, 0.30, -0.40, 0.40, -0.20];
    let sample = WaveformRenderer::sample_at_frame(&samples, 2, 1.5, None);
    assert!(
        sample < -0.45,
        "expected dominant negative channel sample, got {sample}"
    );
}

#[test]
fn supersampled_frame_returns_anchor_sample_for_single_column() {
    let samples = [0.25_f32, 0.75, -0.5, 0.5];
    let sample = WaveformRenderer::supersampled_frame(&samples, 1, samples.len(), 0, 1, Some(0));
    assert!(
        (sample - 0.25).abs() < 1.0e-6,
        "expected first sample fallback, got {sample}"
    );
}

#[test]
fn catmull_rom_matches_linear_midpoint_for_linear_ramp() {
    let sample = WaveformRenderer::catmull_rom(0.0, 1.0, 2.0, 3.0, 0.5);
    assert!(
        (sample - 1.5).abs() < 1.0e-6,
        "expected linear midpoint, got {sample}"
    );
}

#[test]
fn blend_pixel_composites_alpha_and_clamps_coverage() {
    let mut image = solid_image(1, 1, WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 128));
    WaveformRenderer::blend_pixel(&mut image, 1, 0, 0, (200, 100, 50, 255), 1.5);
    let pixel = image.pixels[0];
    assert_eq!(pixel.a(), 255);
    assert!(
        pixel.r() >= 100,
        "expected blended foreground influence, got {}",
        pixel.r()
    );
    assert!(
        pixel.g() >= 60,
        "expected blended green channel, got {}",
        pixel.g()
    );
}

#[test]
fn blend_pixel_skips_zero_coverage() {
    let original = WaveformRgba::from_rgba_unmultiplied(10, 20, 30, 40);
    let mut image = solid_image(1, 1, original);
    WaveformRenderer::blend_pixel(&mut image, 1, 0, 0, (255, 0, 0, 255), 0.0);
    assert_eq!(image.pixels[0], original);
}

#[test]
fn draw_line_aa_handles_vertical_segment_without_oob_writes() {
    let transparent = WaveformRgba::from_rgba_unmultiplied(0, 0, 0, 0);
    let mut image = solid_image(3, 3, transparent);
    WaveformRenderer::draw_line_aa(RasterLineConfig {
        image: &mut image,
        stride: 3,
        width: 3,
        height: 3,
        x0: 1.0,
        y0: 0.0,
        x1: 1.0,
        y1: 2.0,
        fg: (255, 0, 0, 255),
    });
    assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
    assert_eq!(
        image.pixels[0].a(),
        0,
        "unexpected write outside vertical column"
    );
}

#[test]
fn draw_line_aa_handles_steep_segment_with_coverage() {
    let transparent = WaveformRgba::from_rgba_unmultiplied(0, 0, 0, 0);
    let mut image = solid_image(4, 4, transparent);
    WaveformRenderer::draw_line_aa(RasterLineConfig {
        image: &mut image,
        stride: 4,
        width: 4,
        height: 4,
        x0: 0.5,
        y0: 0.0,
        x1: 1.0,
        y1: 3.0,
        fg: (255, 255, 255, 255),
    });
    let lit_pixels = image.pixels.iter().filter(|pixel| pixel.a() > 0).count();
    assert!(
        lit_pixels >= 3,
        "expected multiple covered pixels for steep line, got {lit_pixels}"
    );
}

#[test]
fn paint_line_image_fills_between_center_and_trace() {
    let image = WaveformRenderer::paint_line_image(
        &[0.0_f32, 0.0, 1.0, 1.0],
        1,
        LinePaintConfig {
            width: 4,
            height: 9,
            foreground: WaveformRgba::from_rgb(255, 194, 71),
            background: WaveformRgba::from_rgb(14, 14, 14),
            channel_index: None,
            transient_glow: None,
        },
    );

    let stride = image.size[0];
    let interior = image.pixels[2 * stride + 2];
    let outside = image.pixels[8 * stride + 2];
    assert!(
        interior.a() > 0,
        "expected filled body pixel, got {interior:?}"
    );
    assert_eq!(
        outside.a(),
        0,
        "expected transparent background outside fill"
    );
}

#[test]
fn paint_split_line_image_fills_both_channel_bands() {
    let samples = [0.0_f32, 0.0, 0.0, 0.0, 1.0, -1.0, 1.0, -1.0];
    let image = WaveformRenderer::paint_split_line_image(
        &samples,
        2,
        SplitLinePaintConfig {
            width: 4,
            height: 12,
            foreground: WaveformRgba::from_rgb(255, 194, 71),
            background: WaveformRgba::from_rgb(14, 14, 14),
            transient_glow: None,
        },
    );

    let stride = image.size[0];
    let top_band = image.pixels[1 * stride + 2];
    let bottom_band = image.pixels[9 * stride + 2];
    assert!(
        top_band.a() > 0,
        "expected filled top band, got {top_band:?}"
    );
    assert!(
        bottom_band.a() > 0,
        "expected filled bottom band, got {bottom_band:?}"
    );
}

#[test]
fn transient_glow_brightens_existing_pixels_only() {
    let foreground = WaveformRgba::from_rgb(255, 194, 71);
    let background = WaveformRgba::from_rgb(14, 14, 14);
    let samples = vec![1.0_f32; 64];
    let plain = WaveformRenderer::paint_line_image(
        &samples,
        1,
        LinePaintConfig {
            width: 32,
            height: 9,
            foreground,
            background,
            channel_index: None,
            transient_glow: None,
        },
    );
    let glowed = WaveformRenderer::paint_line_image(
        &samples,
        1,
        LinePaintConfig {
            width: 32,
            height: 9,
            foreground,
            background,
            channel_index: None,
            transient_glow: Some(super::super::super::TransientGlow {
                positions: &[0.5],
                view_start: 0.0,
                view_end: 1.0,
            }),
        },
    );

    let stride = glowed.size[0];
    let glow_idx = 1 * stride + 16;
    let glow_plain = plain.pixels[glow_idx];
    let glow_glowed = glowed.pixels[glow_idx];
    assert!(
        glow_glowed.r() > glow_plain.r()
            || glow_glowed.g() > glow_plain.g()
            || glow_glowed.b() > glow_plain.b()
            || glow_glowed.a() > glow_plain.a(),
        "expected transient glow near highlighted column"
    );

    let transparent_idx = 8 * stride + 16;
    assert_eq!(
        plain.pixels[transparent_idx].a(),
        0,
        "expected baseline transparent background"
    );
    assert_eq!(
        glowed.pixels[transparent_idx].a(),
        0,
        "expected glow to preserve transparent background"
    );
}
