use image::{Rgba, RgbaImage};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct ShotColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug, Clone, Deserialize)]
struct ShotPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct ShotRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ShotPrimitive {
    Rect {
        rect: ShotRect,
        color: ShotColor,
    },
    Circle {
        center: ShotPoint,
        radius: f32,
        color: ShotColor,
    },
    LinearGradient {
        rect: ShotRect,
        start: ShotPoint,
        end: ShotPoint,
        start_color: ShotColor,
        end_color: ShotColor,
    },
    Image {
        rect: ShotRect,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ShotSnapshot {
    viewport_width: u32,
    viewport_height: u32,
    clear_color: ShotColor,
    primitives: Vec<ShotPrimitive>,
}

pub(super) fn rasterize_shot(snapshot: &ShotSnapshot) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(
        snapshot.viewport_width,
        snapshot.viewport_height,
        Rgba([
            snapshot.clear_color.r,
            snapshot.clear_color.g,
            snapshot.clear_color.b,
            snapshot.clear_color.a,
        ]),
    );
    let width = i64::from(snapshot.viewport_width);
    let height = i64::from(snapshot.viewport_height);

    for primitive in &snapshot.primitives {
        match primitive {
            ShotPrimitive::Rect { rect, color } => fill_rect(&mut image, rect, color),
            ShotPrimitive::Circle {
                center,
                radius,
                color,
            } => fill_circle(&mut image, width, height, center, *radius, color),
            ShotPrimitive::LinearGradient {
                rect,
                start,
                end,
                start_color,
                end_color,
            } => fill_linear_gradient(
                &mut image,
                width,
                height,
                rect,
                start,
                end,
                start_color,
                end_color,
            ),
            ShotPrimitive::Image {
                rect,
                width,
                height,
                pixels,
            } => fill_image(&mut image, rect, *width, *height, pixels),
        }
    }
    image
}

fn blend_pixel(target: &mut Rgba<u8>, source: &ShotColor) {
    let source_alpha = source.a as f32 / 255.0;
    if source_alpha <= 0.0 {
        return;
    }
    let target_alpha = target[3] as f32 / 255.0;
    let out_alpha = source_alpha + target_alpha * (1.0 - source_alpha);
    if out_alpha <= 0.0 {
        *target = Rgba([0, 0, 0, 0]);
        return;
    }
    let source_contrib = 1.0 - source_alpha;
    let out_r = (source.r as f32 * source_alpha + target[0] as f32 * target_alpha * source_contrib)
        / out_alpha;
    let out_g = (source.g as f32 * source_alpha + target[1] as f32 * target_alpha * source_contrib)
        / out_alpha;
    let out_b = (source.b as f32 * source_alpha + target[2] as f32 * target_alpha * source_contrib)
        / out_alpha;
    *target = Rgba([
        out_r.clamp(0.0, 255.0).round() as u8,
        out_g.clamp(0.0, 255.0).round() as u8,
        out_b.clamp(0.0, 255.0).round() as u8,
        (out_alpha * 255.0).clamp(0.0, 255.0).round() as u8,
    ]);
}

fn lerp_channel(start: u8, end: u8, amount: f32) -> u8 {
    (start as f32 + ((end as f32 - start as f32) * amount.clamp(0.0, 1.0)))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn lerp_color(start: &ShotColor, end: &ShotColor, amount: f32) -> ShotColor {
    ShotColor {
        r: lerp_channel(start.r, end.r, amount),
        g: lerp_channel(start.g, end.g, amount),
        b: lerp_channel(start.b, end.b, amount),
        a: lerp_channel(start.a, end.a, amount),
    }
}

fn fill_rect(image: &mut RgbaImage, rect: &ShotRect, color: &ShotColor) {
    let width = i64::from(image.width());
    let height = i64::from(image.height());
    let left = rect.x.floor().clamp(0.0, width as f32) as i64;
    let right = (rect.x + rect.width).ceil().clamp(0.0, width as f32) as i64;
    let top = rect.y.floor().clamp(0.0, height as f32) as i64;
    let bottom = (rect.y + rect.height).ceil().clamp(0.0, height as f32) as i64;

    for y in top.max(0)..bottom.min(height) {
        for x in left.max(0)..right.min(width) {
            let pixel =
                image.get_pixel_mut(u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
            blend_pixel(pixel, color);
        }
    }
}

fn fill_circle(
    image: &mut RgbaImage,
    width: i64,
    height: i64,
    center: &ShotPoint,
    radius: f32,
    color: &ShotColor,
) {
    let min_x = (center.x - radius).floor().clamp(0.0, width as f32) as i64;
    let max_x = (center.x + radius).ceil().clamp(0.0, width as f32) as i64;
    let min_y = (center.y - radius).floor().clamp(0.0, height as f32) as i64;
    let max_y = (center.y + radius).ceil().clamp(0.0, height as f32) as i64;
    let radius_sq = radius * radius;

    for y in min_y.max(0)..max_y.min(height) {
        for x in min_x.max(0)..max_x.min(width) {
            let x_offset = x as f32 + 0.5 - center.x;
            let y_offset = y as f32 + 0.5 - center.y;
            if x_offset * x_offset + y_offset * y_offset <= radius_sq {
                let pixel = image
                    .get_pixel_mut(u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
                blend_pixel(pixel, color);
            }
        }
    }
}

fn fill_linear_gradient(
    image: &mut RgbaImage,
    width: i64,
    height: i64,
    rect: &ShotRect,
    start: &ShotPoint,
    end: &ShotPoint,
    start_color: &ShotColor,
    end_color: &ShotColor,
) {
    let left = rect.x.floor().clamp(0.0, width as f32) as i64;
    let right = (rect.x + rect.width).ceil().clamp(0.0, width as f32) as i64;
    let top = rect.y.floor().clamp(0.0, height as f32) as i64;
    let bottom = (rect.y + rect.height).ceil().clamp(0.0, height as f32) as i64;
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;

    for y in top.max(0)..bottom.min(height) {
        for x in left.max(0)..right.min(width) {
            let amount = if len_sq > 0.0 {
                let px = x as f32 + 0.5 - start.x;
                let py = y as f32 + 0.5 - start.y;
                ((px * dx) + (py * dy)) / len_sq
            } else {
                0.0
            };
            let color = lerp_color(start_color, end_color, amount);
            let pixel =
                image.get_pixel_mut(u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
            blend_pixel(pixel, &color);
        }
    }
}

fn fill_image(
    image: &mut RgbaImage,
    rect: &ShotRect,
    image_width: u32,
    image_height: u32,
    pixels: &[u8],
) {
    if image_width == 0 || image_height == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let width = i64::from(image.width());
    let height = i64::from(image.height());
    let left = rect.x.floor().clamp(0.0, width as f32) as i64;
    let right = (rect.x + rect.width).ceil().clamp(0.0, width as f32) as i64;
    let top = rect.y.floor().clamp(0.0, height as f32) as i64;
    let bottom = (rect.y + rect.height).ceil().clamp(0.0, height as f32) as i64;
    let src_width = image_width as usize;
    let src_height = image_height as usize;
    if pixels.len() < src_width.saturating_mul(src_height).saturating_mul(4) {
        return;
    }

    for y in top.max(0)..bottom.min(height) {
        for x in left.max(0)..right.min(width) {
            let norm_x = ((x as f32 + 0.5) - rect.x) / rect.width;
            let norm_y = ((y as f32 + 0.5) - rect.y) / rect.height;
            if !(0.0..=1.0).contains(&norm_x) || !(0.0..=1.0).contains(&norm_y) {
                continue;
            }
            let src_x =
                ((norm_x * image_width as f32).floor() as usize).min(src_width.saturating_sub(1));
            let src_y =
                ((norm_y * image_height as f32).floor() as usize).min(src_height.saturating_sub(1));
            let idx = (src_y * src_width + src_x) * 4;
            let color = ShotColor {
                r: pixels[idx],
                g: pixels[idx + 1],
                b: pixels[idx + 2],
                a: pixels[idx + 3],
            };
            let pixel =
                image.get_pixel_mut(u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
            blend_pixel(pixel, &color);
        }
    }
}
