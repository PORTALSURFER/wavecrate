use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;

use radiant::prelude as ui;
use radiant::widgets::PointerModifiers;
use wavecrate::sample_sources::{
    STARMAP_LAYOUT_UMAP_VERSION, StarmapLayoutLoadRequest, StarmapLayoutLoadResult,
    StarmapLayoutSample, StarmapSourceLayoutRequest,
};
use wavecrate_analysis::aspects::SimilarityAspect;

use crate::native_app::waveform::should_use_file_backed_wav_decode;

use super::{FileEntry, FolderBrowserState, SimilarityAspectStrengths};

const GROUP_CENTERS: [(f32, f32); wavecrate_analysis::aspects::ASPECT_COUNT] = [
    (0.50, 0.50),
    (0.22, 0.36),
    (0.42, 0.28),
    (0.66, 0.36),
    (0.78, 0.62),
];

#[derive(Clone, Copy)]
pub(in crate::native_app) struct StarmapProjection<'a> {
    pub(in crate::native_app) tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) instant_audition_sample_paths: &'a HashSet<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct StarmapStatus {
    pub(in crate::native_app) listed_count: usize,
    pub(in crate::native_app) layout_count: usize,
    pub(in crate::native_app) clustered_count: usize,
    pub(in crate::native_app) cluster_color_count: usize,
}

impl StarmapStatus {
    pub(in crate::native_app) fn incomplete(self) -> bool {
        self.listed_count > 0 && self.layout_count < self.listed_count
    }

    pub(in crate::native_app) fn label(self, prep_running: bool) -> Option<String> {
        if !self.incomplete() {
            return None;
        }
        if prep_running {
            return Some(format!(
                "Preparing Starmap {} / {}",
                self.layout_count, self.listed_count
            ));
        }
        Some(format!(
            "Starmap {} / {}",
            self.layout_count, self.listed_count
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct StarmapLayoutCache {
    signature: Option<u64>,
    pub(super) points_by_file: HashMap<String, StarmapLayoutPoint>,
    listed_count: usize,
    pending_load_signature: Option<u64>,
    loaded_signature: Option<u64>,
    auto_prep_requested_signature: Option<u64>,
}

impl StarmapLayoutCache {
    fn needs_similarity_prep(&self) -> bool {
        self.signature.is_some()
            && self.listed_count > 0
            && self.loaded_signature == self.signature
            && self.pending_load_signature != self.signature
            && self.points_by_file.len() < self.listed_count
            && self.auto_prep_requested_signature != self.signature
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct StarmapLayoutPoint {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) cluster_id: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct StarmapItem {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) x: f32,
    pub(in crate::native_app) y: f32,
    pub(in crate::native_app) color: ui::Rgba8,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) focused: bool,
    pub(in crate::native_app) copy_flash: bool,
    pub(in crate::native_app) similarity_anchor: bool,
    pub(in crate::native_app) instant_audition_ready: bool,
    pub(in crate::native_app) missing: bool,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn prepare_starmap_layout(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let signature = starmap_layout_signature(self.selected_source_id(), snapshot.rows());
        if self.sample_list.starmap_layout.signature == Some(signature) {
            return;
        }
        self.sample_list.starmap_layout = StarmapLayoutCache {
            signature: Some(signature),
            listed_count: snapshot.rows().len(),
            points_by_file: HashMap::new(),
            pending_load_signature: None,
            loaded_signature: None,
            auto_prep_requested_signature: None,
        };
    }

    pub(in crate::native_app) fn invalidate_starmap_layout(&mut self) {
        self.sample_list.starmap_layout = StarmapLayoutCache::default();
    }

    pub(in crate::native_app) fn starmap_sources_needing_similarity_prep(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        self.prepare_starmap_layout(tags_by_file);
        let cache = &self.sample_list.starmap_layout;
        let Some(signature) = cache.signature else {
            return Vec::new();
        };
        if !cache.needs_similarity_prep() {
            return Vec::new();
        }
        let layout_file_ids = cache.points_by_file.keys().cloned().collect::<HashSet<_>>();
        self.sample_list
            .starmap_layout
            .auto_prep_requested_signature = Some(signature);

        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let mut source_ids = Vec::new();
        let mut seen = HashSet::new();
        for file in snapshot.rows() {
            if layout_file_ids.contains(&file.id) {
                continue;
            }
            let Some((source, _)) = self.sample_source_for_file_path(Path::new(&file.id)) else {
                continue;
            };
            let source_id = source.id.as_str().to_string();
            if seen.insert(source_id.clone()) {
                source_ids.push(source_id);
            }
        }
        source_ids
    }

    pub(in crate::native_app) fn take_starmap_layout_load_request(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<StarmapLayoutLoadRequest> {
        let (request, listed_count) = self.starmap_layout_load_request(tags_by_file);
        let signature = request.signature;
        if self.sample_list.starmap_layout.signature != Some(signature) {
            self.sample_list.starmap_layout = StarmapLayoutCache {
                signature: Some(signature),
                listed_count,
                points_by_file: HashMap::new(),
                pending_load_signature: None,
                loaded_signature: None,
                auto_prep_requested_signature: None,
            };
        }
        let cache = &mut self.sample_list.starmap_layout;
        if cache.pending_load_signature == Some(signature)
            || cache.loaded_signature == Some(signature)
            || request.is_empty()
        {
            if request.is_empty() {
                cache.loaded_signature = Some(signature);
            }
            return None;
        }
        cache.pending_load_signature = Some(signature);
        Some(request)
    }

    pub(in crate::native_app) fn apply_starmap_layout_load_result(
        &mut self,
        result: StarmapLayoutLoadResult,
    ) {
        let cache = &mut self.sample_list.starmap_layout;
        if cache.signature != Some(result.signature) {
            return;
        }
        cache.pending_load_signature = None;
        cache.loaded_signature = Some(result.signature);
        match result.result {
            Ok(points) => {
                cache.points_by_file = points
                    .into_iter()
                    .map(|(file_id, point)| {
                        (
                            file_id,
                            StarmapLayoutPoint {
                                x: point.x,
                                y: point.y,
                                cluster_id: point.cluster_id,
                            },
                        )
                    })
                    .collect();
            }
            Err(error) => {
                tracing::debug!(%error, "starmap layout unavailable");
                cache.points_by_file.clear();
            }
        }
    }

    pub(in crate::native_app) fn starmap_projection(
        &self,
        projection: StarmapProjection<'_>,
    ) -> Vec<StarmapItem> {
        let snapshot = self.browser_listing_snapshot(projection.tags_by_file);
        let focused_file_id = self.selected_file_id();
        snapshot
            .rows()
            .iter()
            .map(|file| {
                let instant_audition_ready = instant_audition_ready_for_starmap(
                    file,
                    projection.instant_audition_sample_paths,
                );
                let aspects = self.similarity_aspect_display_strengths_for_file(&file.id);
                let strength = self.similarity_display_strength_for_file(&file.id);
                let group = strongest_enabled_aspect(&aspects, self.similarity_controls());
                let layout_point = self
                    .sample_list
                    .starmap_layout
                    .points_by_file
                    .get(&file.id)
                    .copied();
                let (x, y) = starmap_position(
                    &file.id,
                    group,
                    strength,
                    layout_point.map(|point| (point.x, point.y)),
                );
                StarmapItem {
                    file_id: file.id.clone(),
                    label: file.stem.clone(),
                    x,
                    y,
                    color: starmap_color(group, strength, layout_point),
                    selected: self.is_file_selected(&file.id),
                    focused: focused_file_id == Some(file.id.as_str()),
                    copy_flash: self.copied_file_flash_active(&file.id),
                    similarity_anchor: self.file_is_similarity_anchor(&file.id),
                    instant_audition_ready,
                    missing: file.is_missing(),
                }
            })
            .collect()
    }

    pub(in crate::native_app) fn selected_starmap_position(
        &self,
        projection: StarmapProjection<'_>,
    ) -> Option<(f32, f32)> {
        let selected_file = self.selected_file_id()?;
        self.starmap_projection(projection)
            .into_iter()
            .find(|item| item.file_id == selected_file)
            .map(|item| (item.x, item.y))
    }

    pub(in crate::native_app) fn navigate_starmap_matching_tags(
        &mut self,
        delta: i32,
        extend: bool,
        tags_by_file: &HashMap<String, Vec<String>>,
        instant_audition_sample_paths: &HashSet<String>,
    ) -> Option<String> {
        if delta == 0 || self.rename_active() || !self.selection.selected_file_active() {
            return None;
        }
        self.prepare_starmap_layout(tags_by_file);
        let target = starmap_navigation_target(
            &self.starmap_projection(StarmapProjection {
                tags_by_file,
                instant_audition_sample_paths,
            }),
            self.selection.selected_file_id()?,
            delta,
        )?;
        let visible_ids = self.browser_listing_snapshot(tags_by_file).ids().to_vec();
        if extend {
            self.selection.select_file_with_modifiers(
                target.clone(),
                &visible_ids,
                PointerModifiers {
                    shift: true,
                    ..PointerModifiers::default()
                },
            );
        } else {
            self.selection
                .navigate_file_to_adjacent_visible_id(target.clone())?;
        }
        Some(target)
    }

    pub(in crate::native_app) fn starmap_status(&self) -> StarmapStatus {
        let clustered_count = self
            .sample_list
            .starmap_layout
            .points_by_file
            .values()
            .filter(|point| point.cluster_id.is_some())
            .count();
        let cluster_color_count = if clustered_count == 0 {
            0
        } else {
            clustered_count.min(STARMAP_CLUSTER_PALETTE.len())
        };
        StarmapStatus {
            listed_count: self.sample_list.starmap_layout.listed_count,
            layout_count: self.sample_list.starmap_layout.points_by_file.len(),
            clustered_count,
            cluster_color_count,
        }
    }
}

fn starmap_navigation_target(
    items: &[StarmapItem],
    selected_file_id: &str,
    delta: i32,
) -> Option<String> {
    let current = items
        .iter()
        .find(|item| item.file_id.as_str() == selected_file_id)?;
    let direction = delta.signum() as f32;
    items
        .iter()
        .filter(|item| item.file_id != current.file_id)
        .filter(|item| (item.y - current.y) * direction > f32::EPSILON)
        .min_by(|left, right| {
            starmap_navigation_rank(current, left)
                .total_cmp(&starmap_navigation_rank(current, right))
                .then_with(|| left.file_id.cmp(&right.file_id))
        })
        .map(|item| item.file_id.clone())
}

fn starmap_navigation_rank(current: &StarmapItem, candidate: &StarmapItem) -> f32 {
    let dx = candidate.x - current.x;
    let dy = candidate.y - current.y;
    dx * dx + dy * dy
}

fn instant_audition_ready_for_starmap(
    file: &FileEntry,
    instant_audition_sample_paths: &HashSet<String>,
) -> bool {
    !should_use_file_backed_wav_decode(Path::new(&file.id))
        || instant_audition_sample_paths.contains(&file.id)
}

impl FolderBrowserState {
    fn starmap_layout_load_request(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> (StarmapLayoutLoadRequest, usize) {
        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let listed_count = snapshot.rows().len();
        let mut by_source: HashMap<String, StarmapSourceLayoutRequest> = HashMap::new();
        for file in snapshot.rows() {
            let path = Path::new(&file.id);
            let Some((source, relative_path)) = self.sample_source_for_file_path(path) else {
                continue;
            };
            let sample_id = build_sample_id(source.id.as_str(), &relative_path);
            by_source
                .entry(source.id.as_str().to_string())
                .or_insert_with(|| StarmapSourceLayoutRequest {
                    source,
                    samples: Vec::new(),
                })
                .samples
                .push(StarmapLayoutSample {
                    file_id: file.id.clone(),
                    sample_id,
                });
        }
        let signature = starmap_layout_signature(self.selected_source_id(), snapshot.rows());
        (
            StarmapLayoutLoadRequest {
                signature,
                sources: by_source.into_values().collect(),
            },
            listed_count,
        )
    }
}

fn build_sample_id(source_id: &str, relative_path: &Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}

fn starmap_layout_signature(source_id: &str, files: &[&FileEntry]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_id.hash(&mut hasher);
    STARMAP_LAYOUT_UMAP_VERSION.hash(&mut hasher);
    files.len().hash(&mut hasher);
    for file in files {
        file.id.hash(&mut hasher);
    }
    hasher.finish()
}

fn strongest_enabled_aspect(
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

fn starmap_position(
    file_id: &str,
    group: SimilarityAspect,
    similarity_strength: Option<f32>,
    layout_position: Option<(f32, f32)>,
) -> (f32, f32) {
    if let Some(position) = layout_position {
        return position;
    }
    let (jx, jy) = stable_jitter(file_id);
    let (gx, gy) = GROUP_CENTERS[group.index()];
    let strength = similarity_strength.unwrap_or(0.0);
    let spread = 0.31 - strength.clamp(0.0, 1.0) * 0.13;
    (
        (gx + (jx - 0.5) * spread).clamp(0.04, 0.96),
        (gy + (jy - 0.5) * spread).clamp(0.06, 0.94),
    )
}

fn stable_jitter(file_id: &str) -> (f32, f32) {
    let mut first = std::collections::hash_map::DefaultHasher::new();
    file_id.hash(&mut first);
    let mut second = std::collections::hash_map::DefaultHasher::new();
    // Preserve the historical salt so fallback Starmap placement does not shift.
    "sample-map-y".hash(&mut second);
    file_id.hash(&mut second);
    (
        unit_from_hash(first.finish()),
        unit_from_hash(second.finish()),
    )
}

fn unit_from_hash(hash: u64) -> f32 {
    (hash as f64 / u64::MAX as f64) as f32
}

fn starmap_color(
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

const STARMAP_CLUSTER_PALETTE: [ui::Rgba8; 5] = [
    ui::Rgba8::new(255, 55, 96, 230),
    ui::Rgba8::new(114, 235, 184, 230),
    ui::Rgba8::new(255, 179, 92, 230),
    ui::Rgba8::new(186, 91, 255, 230),
    ui::Rgba8::new(57, 187, 245, 230),
];

#[cfg(test)]
mod tests {
    use super::*;
    use wavecrate::sample_sources::SampleSource;

    #[test]
    fn starmap_position_is_stable_and_bounded() {
        let first = starmap_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8), None);
        let second = starmap_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8), None);

        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first.0));
        assert!((0.0..=1.0).contains(&first.1));
    }

    #[test]
    fn starmap_position_uses_normalized_layout_when_available() {
        let position = starmap_position(
            "kick.wav",
            SimilarityAspect::Spectrum,
            Some(0.8),
            Some((0.25, 0.75)),
        );

        assert_eq!(position, (0.25, 0.75));
    }

    #[test]
    fn starmap_position_falls_back_when_layout_is_missing() {
        let fallback = starmap_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8), None);
        let layout = starmap_position(
            "kick.wav",
            SimilarityAspect::Spectrum,
            Some(0.8),
            Some((0.25, 0.75)),
        );

        assert_ne!(fallback, layout);
        assert!((0.0..=1.0).contains(&fallback.0));
        assert!((0.0..=1.0).contains(&fallback.1));
    }

    #[test]
    fn starmap_color_prefers_similarity_cluster_color() {
        let cluster_color = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.16,
                y: 0.46,
                cluster_id: Some(1),
            }),
        );
        let aspect_color = starmap_color(SimilarityAspect::Spectrum, Some(0.5), None);

        assert_ne!(cluster_color, aspect_color);
        assert_eq!(cluster_color.a, 210);
    }

    #[test]
    fn starmap_cluster_colors_fade_by_layout_position() {
        let left = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.16,
                y: 0.46,
                cluster_id: Some(1),
            }),
        );
        let nearby = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.20,
                y: 0.48,
                cluster_id: Some(37),
            }),
        );
        let far = starmap_color(
            SimilarityAspect::Spectrum,
            Some(0.5),
            Some(StarmapLayoutPoint {
                x: 0.84,
                y: 0.62,
                cluster_id: Some(37),
            }),
        );

        assert!(
            color_distance(left, nearby) < color_distance(left, far),
            "nearby clustered samples should have more similar colors than distant samples"
        );
    }

    #[test]
    fn selected_starmap_position_uses_current_filtered_projection() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        std::fs::write(&kick, []).expect("write sample");
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let kick_id = kick.to_string_lossy().to_string();
        browser.select_file(kick_id.clone());
        let tags_by_file = HashMap::new();
        browser.prepare_starmap_layout(&tags_by_file);

        let position = browser.selected_starmap_position(StarmapProjection {
            tags_by_file: &tags_by_file,
            instant_audition_sample_paths: &HashSet::new(),
        });

        assert!(position.is_some());
        let projection = browser.starmap_projection(StarmapProjection {
            tags_by_file: &tags_by_file,
            instant_audition_sample_paths: &HashSet::new(),
        });
        let selected = projection
            .iter()
            .find(|item| item.file_id == kick_id)
            .expect("selected map item");
        assert_eq!(position, Some((selected.x, selected.y)));
        assert!(selected.focused);
    }

    #[test]
    fn starmap_keyboard_navigation_uses_map_position_not_list_order() {
        let root = tempfile::tempdir().expect("source root");
        let alpha = root.path().join("alpha.wav");
        let beta = root.path().join("beta.wav");
        let close_below = root.path().join("close_below.wav");
        std::fs::write(&alpha, []).expect("write alpha");
        std::fs::write(&beta, []).expect("write beta");
        std::fs::write(&close_below, []).expect("write close");
        let alpha_id = alpha.to_string_lossy().to_string();
        let beta_id = beta.to_string_lossy().to_string();
        let close_below_id = close_below.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();
        browser.prepare_starmap_layout(&tags_by_file);
        browser.sample_list.starmap_layout.points_by_file = HashMap::from([
            (
                alpha_id.clone(),
                StarmapLayoutPoint {
                    x: 0.50,
                    y: 0.50,
                    cluster_id: None,
                },
            ),
            (
                beta_id.clone(),
                StarmapLayoutPoint {
                    x: 0.50,
                    y: 0.92,
                    cluster_id: None,
                },
            ),
            (
                close_below_id.clone(),
                StarmapLayoutPoint {
                    x: 0.52,
                    y: 0.58,
                    cluster_id: None,
                },
            ),
        ]);
        browser.select_file(alpha_id.clone());

        let down = browser.navigate_starmap_matching_tags(1, false, &tags_by_file, &HashSet::new());
        let up = browser.navigate_starmap_matching_tags(-1, false, &tags_by_file, &HashSet::new());

        assert_eq!(
            down,
            Some(close_below_id),
            "map navigation should pick the closest lower map node, not the next filename row"
        );
        assert_eq!(up, Some(alpha_id));
    }

    #[test]
    fn starmap_projection_matches_filtered_browser_listing() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("deep_kick.wav");
        let snare = root.path().join("deep_snare.wav");
        let hat = root.path().join("bright_hat.wav");
        std::fs::write(&kick, []).expect("write kick");
        std::fs::write(&snare, []).expect("write snare");
        std::fs::write(&hat, []).expect("write hat");
        let kick_id = kick.to_string_lossy().to_string();
        let snare_id = snare.to_string_lossy().to_string();
        let hat_id = hat.to_string_lossy().to_string();
        let tags_by_file = HashMap::from([
            (kick_id.clone(), vec![String::from("drum")]),
            (snare_id.clone(), vec![String::from("drum")]),
            (hat_id.clone(), vec![String::from("metal")]),
        ]);
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);

        browser.apply_name_filter_input(radiant::widgets::TextInputMessage::Changed {
            value: String::from("deep"),
        });
        browser.apply_tag_filter_input(radiant::widgets::TextInputMessage::Changed {
            value: String::from("drum"),
        });
        browser.prepare_starmap_layout(&tags_by_file);

        let listing_ids = browser
            .browser_listing_snapshot(&tags_by_file)
            .ids()
            .to_vec();
        let map_ids = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| item.file_id)
            .collect::<Vec<_>>();

        assert_eq!(listing_ids, vec![kick_id, snare_id]);
        assert_eq!(
            map_ids, listing_ids,
            "starmap mode must project exactly the same filtered files as list mode"
        );
    }

    #[test]
    fn starmap_projection_uses_full_filtered_listing_not_virtual_list_window() {
        let root = tempfile::tempdir().expect("source root");
        let files = (0..32)
            .map(|index| root.path().join(format!("drum_{index:02}.wav")))
            .collect::<Vec<_>>();
        for file in &files {
            std::fs::write(file, []).expect("write sample");
        }
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
            offset_y: 0.0,
            row_height: 22.0,
            window: radiant::prelude::VirtualListWindow {
                total_items: 32,
                viewport_start: 0,
                viewport_end: 8,
                window_start: 0,
                window_end: 8,
            },
        });
        let tags_by_file = HashMap::new();
        let cached_sample_paths = HashSet::new();

        let visible = browser.visible_samples(
            crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery {
                tags_by_file: &tags_by_file,
                cached_sample_paths: &cached_sample_paths,
            },
        );
        let map_ids = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| item.file_id)
            .collect::<Vec<_>>();
        let listing_ids = browser
            .browser_listing_snapshot(&tags_by_file)
            .ids()
            .to_vec();

        assert!(visible.rows.len() < visible.total_count);
        assert_eq!(visible.rows.len(), 8);
        assert_eq!(visible.total_count, 32);
        assert_eq!(
            map_ids, listing_ids,
            "starmap must include the full filtered listing, not only virtualized list rows"
        );
    }

    #[test]
    fn starmap_projection_marks_cold_long_wavs_as_not_audition_ready() {
        let root = tempfile::tempdir().expect("source root");
        let short = root.path().join("short.wav");
        let long = root.path().join("long.wav");
        std::fs::write(&short, []).expect("write short sample");
        std::fs::write(&long, vec![0_u8; 2048]).expect("write long sample");
        let short_id = short.to_string_lossy().to_string();
        let long_id = long.to_string_lossy().to_string();
        let browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();

        let cold_items = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .map(|item| (item.file_id, item.instant_audition_ready))
            .collect::<Vec<_>>();
        let ready_items = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::from([long_id.clone()]),
            })
            .into_iter()
            .map(|item| (item.file_id, item.instant_audition_ready))
            .collect::<Vec<_>>();

        assert_eq!(
            cold_items,
            vec![(long_id.clone(), false), (short_id.clone(), true)]
        );
        assert_eq!(ready_items, vec![(long_id, true), (short_id, true)]);
    }

    #[test]
    fn starmap_projection_groups_by_enabled_similarity_aspects() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        let snare = root.path().join("snare.wav");
        std::fs::write(&kick, []).expect("write kick");
        std::fs::write(&snare, []).expect("write snare");
        let kick_id = kick.to_string_lossy().to_string();
        let snare_id = snare.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let mut aspects = [None; wavecrate_analysis::aspects::ASPECT_COUNT];
        aspects[SimilarityAspect::Spectrum.index()] = Some(0.6);
        aspects[SimilarityAspect::Timbre.index()] = Some(1.0);
        browser.set_similarity_scores_with_aspects(
            kick_id,
            HashMap::from([(snare_id.clone(), 0.9)]),
            HashMap::from([(snare_id.clone(), aspects)]),
        );
        let tags_by_file = HashMap::new();

        let timbre_color = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .find(|item| item.file_id == snare_id.as_str())
            .expect("snare map item")
            .color;

        let mut controls = browser.similarity_controls().clone();
        controls.set_aspect_enabled(SimilarityAspect::Timbre, false);
        browser.set_similarity_controls(controls);
        let spectrum_color = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .find(|item| item.file_id == snare_id.as_str())
            .expect("snare map item after disabling timbre")
            .color;

        assert_eq!(
            (timbre_color.r, timbre_color.g, timbre_color.b),
            (255, 142, 56)
        );
        assert_eq!(
            (spectrum_color.r, spectrum_color.g, spectrum_color.b),
            (239, 216, 66)
        );
    }

    #[test]
    fn starmap_projection_marks_all_selected_list_items() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        let snare = root.path().join("snare.wav");
        let hat = root.path().join("hat.wav");
        std::fs::write(&kick, []).expect("write kick");
        std::fs::write(&snare, []).expect("write snare");
        std::fs::write(&hat, []).expect("write hat");
        let kick_id = kick.to_string_lossy().to_string();
        let snare_id = snare.to_string_lossy().to_string();
        let hat_id = hat.to_string_lossy().to_string();
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let tags_by_file = HashMap::new();

        browser.select_file(kick_id.clone());
        browser.select_file_with_modifiers(
            snare_id.clone(),
            radiant::widgets::PointerModifiers {
                command: true,
                ..radiant::widgets::PointerModifiers::default()
            },
        );

        let selected_map_items = browser
            .starmap_projection(StarmapProjection {
                tags_by_file: &tags_by_file,
                instant_audition_sample_paths: &HashSet::new(),
            })
            .into_iter()
            .filter(|item| item.selected)
            .collect::<Vec<_>>();
        let selected_map_ids = selected_map_items
            .iter()
            .map(|item| item.file_id.clone())
            .collect::<Vec<_>>();
        let focused_map_ids = selected_map_items
            .iter()
            .filter(|item| item.focused)
            .map(|item| item.file_id.clone())
            .collect::<Vec<_>>();

        assert_eq!(selected_map_ids, vec![kick_id, snare_id.clone()]);
        assert_eq!(focused_map_ids, vec![snare_id]);
        assert!(!selected_map_ids.contains(&hat_id));
    }

    #[test]
    fn starmap_status_reports_incomplete_layout_coverage() {
        let status = StarmapStatus {
            listed_count: 12,
            layout_count: 5,
            clustered_count: 2,
            cluster_color_count: 2,
        };

        assert!(status.incomplete());
        assert_eq!(
            status.label(true),
            Some(String::from("Preparing Starmap 5 / 12"))
        );
        assert_eq!(status.label(false), Some(String::from("Starmap 5 / 12")));
    }

    #[test]
    fn complete_starmap_status_stays_silent() {
        let status = StarmapStatus {
            listed_count: 12,
            layout_count: 12,
            clustered_count: 8,
            cluster_color_count: 4,
        };

        assert!(!status.incomplete());
        assert_eq!(status.label(true), None);
    }

    #[test]
    fn incomplete_starmap_layout_requests_similarity_prep_once_per_signature() {
        let mut cache = StarmapLayoutCache {
            signature: Some(42),
            listed_count: 2,
            points_by_file: HashMap::from([(
                String::from("a.wav"),
                StarmapLayoutPoint {
                    x: 0.2,
                    y: 0.3,
                    cluster_id: None,
                },
            )]),
            pending_load_signature: None,
            loaded_signature: Some(42),
            auto_prep_requested_signature: None,
        };

        assert!(cache.needs_similarity_prep());

        cache.auto_prep_requested_signature = Some(42);

        assert!(!cache.needs_similarity_prep());
    }

    #[test]
    fn complete_or_empty_starmap_layout_does_not_request_similarity_prep() {
        let complete = StarmapLayoutCache {
            signature: Some(7),
            listed_count: 1,
            points_by_file: HashMap::from([(
                String::from("a.wav"),
                StarmapLayoutPoint {
                    x: 0.2,
                    y: 0.3,
                    cluster_id: None,
                },
            )]),
            pending_load_signature: None,
            loaded_signature: Some(7),
            auto_prep_requested_signature: None,
        };
        let empty_listing = StarmapLayoutCache {
            signature: Some(8),
            listed_count: 0,
            points_by_file: HashMap::new(),
            pending_load_signature: None,
            loaded_signature: Some(8),
            auto_prep_requested_signature: None,
        };

        assert!(!complete.needs_similarity_prep());
        assert!(!empty_listing.needs_similarity_prep());
    }

    #[test]
    fn strongest_enabled_aspect_uses_similarity_strengths() {
        let mut aspects = [None; wavecrate_analysis::aspects::ASPECT_COUNT];
        aspects[SimilarityAspect::Spectrum.index()] = Some(0.2);
        aspects[SimilarityAspect::Timbre.index()] = Some(0.9);

        assert_eq!(
            strongest_enabled_aspect(
                &aspects,
                &wavecrate::sample_sources::config::SimilarityAspectSettings::default(),
            ),
            SimilarityAspect::Timbre
        );
    }

    fn color_distance(left: ui::Rgba8, right: ui::Rgba8) -> u16 {
        u16::from(left.r.abs_diff(right.r))
            + u16::from(left.g.abs_diff(right.g))
            + u16::from(left.b.abs_diff(right.b))
    }
}
