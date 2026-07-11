use radiant::prelude as ui;
use wavecrate_analysis::aspects::SimilarityAspect;

use super::super::SimilarityAspectStrengths;
use super::StarmapLayoutPoint;

pub(super) fn strongest_enabled_aspect(
    aspects: &SimilarityAspectStrengths,
    controls: &wavecrate::sample_sources::config::SimilarityAspectSettings,
) -> SimilarityAspect {
    let enabled = controls.aspect_enabled_flags();
    SimilarityAspect::ORDER
        .iter()
        .copied()
        .filter(|aspect| enabled[aspect.index()])
        .filter(|aspect| *aspect != SimilarityAspect::Overall)
        .max_by(|left, right| {
            aspect_strength(aspects, *left)
                .partial_cmp(&aspect_strength(aspects, *right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(SimilarityAspect::Overall)
}

fn aspect_strength(aspects: &SimilarityAspectStrengths, aspect: SimilarityAspect) -> f32 {
    aspects
        .get(aspect.index())
        .copied()
        .flatten()
        .unwrap_or(0.0)
}

pub(super) fn starmap_position(layout_point: StarmapLayoutPoint) -> (f32, f32) {
    (layout_point.x, layout_point.y)
}

pub(super) fn starmap_color(
    group: SimilarityAspect,
    strength: Option<f32>,
    layout_point: Option<StarmapLayoutPoint>,
) -> ui::Rgba8 {
    if let Some(point) = layout_point
        && point.cluster_id.is_some()
    {
        return starmap_cluster_color((point.x, point.y), strength);
    }
    let alpha = (150.0 + strength.unwrap_or(0.35).clamp(0.0, 1.0) * 90.0) as u8;
    match group {
        SimilarityAspect::Overall => ui::Rgba8::new(122, 226, 96, alpha),
        SimilarityAspect::Spectrum => ui::Rgba8::new(239, 216, 66, alpha),
        SimilarityAspect::Timbre => ui::Rgba8::new(255, 142, 56, alpha),
        SimilarityAspect::Pitch => ui::Rgba8::new(255, 55, 96, alpha),
        SimilarityAspect::Amplitude => ui::Rgba8::new(57, 187, 245, alpha),
    }
}

fn starmap_cluster_color(position: (f32, f32), strength: Option<f32>) -> ui::Rgba8 {
    let alpha = (180.0 + strength.unwrap_or(0.45).clamp(0.0, 1.0) * 60.0) as u8;
    blended_starmap_cluster_color(position).with_alpha(alpha)
}

pub(in crate::native_app) fn starmap_cluster_palette_color(index: usize) -> ui::Rgba8 {
    STARMAP_CLUSTER_PALETTE[index % STARMAP_CLUSTER_PALETTE.len()]
}

fn blended_starmap_cluster_color(position: (f32, f32)) -> ui::Rgba8 {
    let mut total = 0.0;
    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;
    for anchor in STARMAP_CLUSTER_COLOR_ANCHORS {
        let dx = position.0 - anchor.x;
        let dy = position.1 - anchor.y;
        let weight = 1.0 / (dx * dx + dy * dy + 0.025).powf(1.6);
        total += weight;
        red += f32::from(anchor.color.r) * weight;
        green += f32::from(anchor.color.g) * weight;
        blue += f32::from(anchor.color.b) * weight;
    }
    ui::Rgba8::new(
        blended_color_channel(red, total),
        blended_color_channel(green, total),
        blended_color_channel(blue, total),
        230,
    )
}

fn blended_color_channel(weighted: f32, total: f32) -> u8 {
    if total <= f32::EPSILON {
        return 0;
    }
    (weighted / total).round().clamp(0.0, 255.0) as u8
}

#[derive(Clone, Copy)]
struct StarmapClusterColorAnchor {
    x: f32,
    y: f32,
    color: ui::Rgba8,
}

const STARMAP_CLUSTER_COLOR_ANCHORS: [StarmapClusterColorAnchor; 5] = [
    StarmapClusterColorAnchor {
        x: 0.16,
        y: 0.46,
        color: STARMAP_CLUSTER_PALETTE[0],
    },
    StarmapClusterColorAnchor {
        x: 0.36,
        y: 0.24,
        color: STARMAP_CLUSTER_PALETTE[1],
    },
    StarmapClusterColorAnchor {
        x: 0.52,
        y: 0.52,
        color: STARMAP_CLUSTER_PALETTE[2],
    },
    StarmapClusterColorAnchor {
        x: 0.68,
        y: 0.34,
        color: STARMAP_CLUSTER_PALETTE[3],
    },
    StarmapClusterColorAnchor {
        x: 0.84,
        y: 0.62,
        color: STARMAP_CLUSTER_PALETTE[4],
    },
];

pub(super) const STARMAP_CLUSTER_PALETTE: [ui::Rgba8; 5] = [
    ui::Rgba8::new(255, 55, 96, 230),
    ui::Rgba8::new(114, 235, 184, 230),
    ui::Rgba8::new(255, 179, 92, 230),
    ui::Rgba8::new(186, 91, 255, 230),
    ui::Rgba8::new(57, 187, 245, 230),
];
