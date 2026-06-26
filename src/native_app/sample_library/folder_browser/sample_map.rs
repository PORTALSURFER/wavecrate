use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;

use radiant::prelude as ui;
use rusqlite::params_from_iter;
use wavecrate::sample_sources::{SampleSource, SourceDatabase, SourceDatabaseConnectionRole};
use wavecrate_analysis::aspects::SimilarityAspect;
use wavecrate_analysis::similarity::SIMILARITY_MODEL_ID;

use crate::native_app::sample_library::similarity_prep::NATIVE_SIMILARITY_UMAP_VERSION;

use super::{FileEntry, FolderBrowserState, SimilarityAspectStrengths};

const GROUP_CENTERS: [(f32, f32); wavecrate_analysis::aspects::ASPECT_COUNT] = [
    (0.50, 0.50),
    (0.22, 0.36),
    (0.42, 0.28),
    (0.66, 0.36),
    (0.78, 0.62),
];

#[derive(Clone, Copy)]
pub(in crate::native_app) struct SampleMapProjection<'a> {
    pub(in crate::native_app) tags_by_file: &'a HashMap<String, Vec<String>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct SampleMapStatus {
    pub(in crate::native_app) listed_count: usize,
    pub(in crate::native_app) layout_count: usize,
    pub(in crate::native_app) clustered_count: usize,
}

impl SampleMapStatus {
    pub(in crate::native_app) fn incomplete(self) -> bool {
        self.listed_count > 0 && self.layout_count < self.listed_count
    }

    pub(in crate::native_app) fn label(self, prep_running: bool) -> Option<String> {
        if !self.incomplete() {
            return None;
        }
        if prep_running {
            return Some(format!(
                "Preparing similarity map {} / {}",
                self.layout_count, self.listed_count
            ));
        }
        Some(format!(
            "Similarity map {} / {}",
            self.layout_count, self.listed_count
        ))
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct SampleMapLayoutCache {
    signature: Option<u64>,
    pub(super) points_by_file: HashMap<String, SampleMapLayoutPoint>,
    listed_count: usize,
    auto_prep_requested_signature: Option<u64>,
}

impl SampleMapLayoutCache {
    fn needs_similarity_prep(&self) -> bool {
        self.signature.is_some()
            && self.listed_count > 0
            && self.points_by_file.len() < self.listed_count
            && self.auto_prep_requested_signature != self.signature
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SampleMapLayoutPoint {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) cluster_id: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SampleMapItem {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) x: f32,
    pub(in crate::native_app) y: f32,
    pub(in crate::native_app) color: ui::Rgba8,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) focused: bool,
    pub(in crate::native_app) similarity_anchor: bool,
    pub(in crate::native_app) missing: bool,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn prepare_sample_map_layout(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        let snapshot = self.browser_listing_snapshot(tags_by_file);
        let signature = sample_map_layout_signature(self.selected_source_id(), snapshot.rows());
        if self.sample_list.sample_map_layout.signature == Some(signature) {
            return;
        }
        let positions = match load_sample_map_layout_positions(self, snapshot.rows()) {
            Ok(positions) => positions,
            Err(err) => {
                tracing::debug!(error = %err, "sample map layout unavailable");
                HashMap::new()
            }
        };
        self.sample_list.sample_map_layout = SampleMapLayoutCache {
            signature: Some(signature),
            listed_count: snapshot.rows().len(),
            points_by_file: positions,
            auto_prep_requested_signature: None,
        };
    }

    pub(in crate::native_app) fn invalidate_sample_map_layout(&mut self) {
        self.sample_list.sample_map_layout = SampleMapLayoutCache::default();
    }

    pub(in crate::native_app) fn sample_map_sources_needing_similarity_prep(
        &mut self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        self.prepare_sample_map_layout(tags_by_file);
        let cache = &self.sample_list.sample_map_layout;
        let Some(signature) = cache.signature else {
            return Vec::new();
        };
        if !cache.needs_similarity_prep() {
            return Vec::new();
        }
        let layout_file_ids = cache.points_by_file.keys().cloned().collect::<HashSet<_>>();
        self.sample_list
            .sample_map_layout
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

    pub(in crate::native_app) fn sample_map_projection(
        &self,
        projection: SampleMapProjection<'_>,
    ) -> Vec<SampleMapItem> {
        let snapshot = self.browser_listing_snapshot(projection.tags_by_file);
        let focused_file_id = self.selected_file_id();
        snapshot
            .rows()
            .iter()
            .map(|file| {
                let aspects = self.similarity_aspect_display_strengths_for_file(&file.id);
                let strength = self.similarity_display_strength_for_file(&file.id);
                let group = strongest_enabled_aspect(&aspects, self.similarity_controls());
                let layout_point = self
                    .sample_list
                    .sample_map_layout
                    .points_by_file
                    .get(&file.id)
                    .copied();
                let (x, y) = sample_map_position(
                    &file.id,
                    group,
                    strength,
                    layout_point.map(|point| (point.x, point.y)),
                );
                SampleMapItem {
                    file_id: file.id.clone(),
                    label: file.stem.clone(),
                    x,
                    y,
                    color: sample_map_color(
                        group,
                        strength,
                        layout_point.and_then(|point| point.cluster_id),
                    ),
                    selected: self.is_file_selected(&file.id),
                    focused: focused_file_id == Some(file.id.as_str()),
                    similarity_anchor: self.file_is_similarity_anchor(&file.id),
                    missing: file.is_missing(),
                }
            })
            .collect()
    }

    pub(in crate::native_app) fn selected_sample_map_position(
        &self,
        projection: SampleMapProjection<'_>,
    ) -> Option<(f32, f32)> {
        let selected_file = self.selected_file_id()?;
        self.sample_map_projection(projection)
            .into_iter()
            .find(|item| item.file_id == selected_file)
            .map(|item| (item.x, item.y))
    }

    pub(in crate::native_app) fn sample_map_status(&self) -> SampleMapStatus {
        SampleMapStatus {
            listed_count: self.sample_list.sample_map_layout.listed_count,
            layout_count: self.sample_list.sample_map_layout.points_by_file.len(),
            clustered_count: self
                .sample_list
                .sample_map_layout
                .points_by_file
                .values()
                .filter(|point| point.cluster_id.is_some())
                .count(),
        }
    }
}

fn load_sample_map_layout_positions(
    browser: &FolderBrowserState,
    files: &[&FileEntry],
) -> Result<HashMap<String, SampleMapLayoutPoint>, String> {
    let mut by_source: HashMap<String, SourceLayoutRequest> = HashMap::new();
    for file in files {
        let path = Path::new(&file.id);
        let Some((source, relative_path)) = browser.sample_source_for_file_path(path) else {
            continue;
        };
        let sample_id = build_sample_id(source.id.as_str(), &relative_path);
        by_source
            .entry(source.id.as_str().to_string())
            .or_insert_with(|| SourceLayoutRequest {
                source,
                samples: Vec::new(),
            })
            .samples
            .push(FileLayoutSample {
                file_id: file.id.clone(),
                sample_id,
            });
    }

    let mut raw_points = HashMap::new();
    for request in by_source.values() {
        load_source_layout_positions(request, &mut raw_points)?;
    }
    Ok(normalized_layout_points(raw_points))
}

#[derive(Clone, Debug)]
struct SourceLayoutRequest {
    source: SampleSource,
    samples: Vec<FileLayoutSample>,
}

#[derive(Clone, Debug)]
struct FileLayoutSample {
    file_id: String,
    sample_id: String,
}

fn load_source_layout_positions(
    request: &SourceLayoutRequest,
    positions: &mut HashMap<String, RawSampleMapLayoutPoint>,
) -> Result<(), String> {
    let database_root = request
        .source
        .database_root()
        .map_err(|err| format!("Resolve source metadata location failed: {err}"))?;
    let conn = SourceDatabase::open_connection_with_role_and_database_root(
        &request.source.root,
        database_root,
        SourceDatabaseConnectionRole::UiRead,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))?;
    let file_by_sample_id = request
        .samples
        .iter()
        .map(|sample| (sample.sample_id.as_str(), sample.file_id.as_str()))
        .collect::<HashMap<_, _>>();
    for chunk in request.samples.chunks(256) {
        let mut query = String::from(
            "SELECT layout_umap.sample_id, layout_umap.x, layout_umap.y, hdbscan_clusters.cluster_id \
             FROM layout_umap \
             LEFT JOIN hdbscan_clusters \
                ON layout_umap.sample_id = hdbscan_clusters.sample_id \
               AND hdbscan_clusters.model_id = ?1 \
               AND hdbscan_clusters.method = ?3 \
               AND hdbscan_clusters.umap_version = ?2 \
             WHERE layout_umap.model_id = ?1 AND layout_umap.umap_version = ?2 AND layout_umap.sample_id IN (",
        );
        query.push_str(
            &std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(","),
        );
        query.push(')');

        let mut params = Vec::with_capacity(chunk.len() + 3);
        params.push(SIMILARITY_MODEL_ID.to_string());
        params.push(NATIVE_SIMILARITY_UMAP_VERSION.to_string());
        params.push(String::from("umap"));
        params.extend(chunk.iter().map(|sample| sample.sample_id.clone()));

        let mut statement = conn
            .prepare(&query)
            .map_err(|err| format!("Prepare map layout query failed: {err}"))?;
        let rows = statement
            .query_map(params_from_iter(params.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f32>(1)?,
                    row.get::<_, f32>(2)?,
                    row.get::<_, Option<i32>>(3)?,
                ))
            })
            .map_err(|err| format!("Query map layout failed: {err}"))?;
        for row in rows {
            let (sample_id, x, y, cluster_id) =
                row.map_err(|err| format!("Decode map layout row failed: {err}"))?;
            let Some(file_id) = file_by_sample_id.get(sample_id.as_str()) else {
                continue;
            };
            positions.insert(
                (*file_id).to_string(),
                RawSampleMapLayoutPoint { x, y, cluster_id },
            );
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RawSampleMapLayoutPoint {
    x: f32,
    y: f32,
    cluster_id: Option<i32>,
}

fn build_sample_id(source_id: &str, relative_path: &Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}

fn sample_map_layout_signature(source_id: &str, files: &[&FileEntry]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_id.hash(&mut hasher);
    NATIVE_SIMILARITY_UMAP_VERSION.hash(&mut hasher);
    files.len().hash(&mut hasher);
    for file in files {
        file.id.hash(&mut hasher);
    }
    hasher.finish()
}

fn normalized_layout_points(
    raw_points: HashMap<String, RawSampleMapLayoutPoint>,
) -> HashMap<String, SampleMapLayoutPoint> {
    if raw_points.is_empty() {
        return HashMap::new();
    }
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for point in raw_points.values().copied() {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    raw_points
        .into_iter()
        .map(|(file_id, point)| {
            (
                file_id,
                SampleMapLayoutPoint {
                    x: normalize_layout_axis(point.x, min_x, max_x, 0.04, 0.96),
                    y: normalize_layout_axis(point.y, min_y, max_y, 0.06, 0.94),
                    cluster_id: point.cluster_id,
                },
            )
        })
        .collect()
}

fn normalize_layout_axis(value: f32, min: f32, max: f32, out_min: f32, out_max: f32) -> f32 {
    if !value.is_finite() || !min.is_finite() || !max.is_finite() {
        return (out_min + out_max) * 0.5;
    }
    let span = max - min;
    if span.abs() <= f32::EPSILON {
        return (out_min + out_max) * 0.5;
    }
    let unit = ((value - min) / span).clamp(0.0, 1.0);
    out_min + (out_max - out_min) * unit
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

fn sample_map_position(
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

fn sample_map_color(
    group: SimilarityAspect,
    strength: Option<f32>,
    cluster_id: Option<i32>,
) -> ui::Rgba8 {
    if let Some(cluster_id) = cluster_id {
        return sample_map_cluster_color(cluster_id, strength);
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

fn sample_map_cluster_color(cluster_id: i32, strength: Option<f32>) -> ui::Rgba8 {
    let alpha = (180.0 + strength.unwrap_or(0.45).clamp(0.0, 1.0) * 60.0) as u8;
    sample_map_cluster_palette_color(
        cluster_id.rem_euclid(SAMPLE_MAP_CLUSTER_PALETTE.len() as i32) as usize,
    )
    .with_alpha(alpha)
}

pub(in crate::native_app) fn sample_map_cluster_palette_color(index: usize) -> ui::Rgba8 {
    SAMPLE_MAP_CLUSTER_PALETTE[index % SAMPLE_MAP_CLUSTER_PALETTE.len()]
}

const SAMPLE_MAP_CLUSTER_PALETTE: [ui::Rgba8; 12] = [
    ui::Rgba8::new(255, 55, 96, 230),
    ui::Rgba8::new(57, 187, 245, 230),
    ui::Rgba8::new(239, 216, 66, 230),
    ui::Rgba8::new(114, 235, 184, 230),
    ui::Rgba8::new(255, 142, 56, 230),
    ui::Rgba8::new(186, 91, 255, 230),
    ui::Rgba8::new(255, 119, 210, 230),
    ui::Rgba8::new(142, 255, 90, 230),
    ui::Rgba8::new(255, 179, 92, 230),
    ui::Rgba8::new(92, 255, 230, 230),
    ui::Rgba8::new(255, 92, 92, 230),
    ui::Rgba8::new(168, 190, 255, 230),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_map_position_is_stable_and_bounded() {
        let first = sample_map_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8), None);
        let second = sample_map_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8), None);

        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first.0));
        assert!((0.0..=1.0).contains(&first.1));
    }

    #[test]
    fn sample_map_position_uses_normalized_layout_when_available() {
        let position = sample_map_position(
            "kick.wav",
            SimilarityAspect::Spectrum,
            Some(0.8),
            Some((0.25, 0.75)),
        );

        assert_eq!(position, (0.25, 0.75));
    }

    #[test]
    fn sample_map_position_falls_back_when_layout_is_missing() {
        let fallback = sample_map_position("kick.wav", SimilarityAspect::Spectrum, Some(0.8), None);
        let layout = sample_map_position(
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
    fn normalized_layout_positions_fill_map_domain() {
        let positions = normalized_layout_points(HashMap::from([
            (
                String::from("a.wav"),
                RawSampleMapLayoutPoint {
                    x: -1.0,
                    y: 2.0,
                    cluster_id: Some(3),
                },
            ),
            (
                String::from("b.wav"),
                RawSampleMapLayoutPoint {
                    x: 1.0,
                    y: 6.0,
                    cluster_id: Some(7),
                },
            ),
        ]));

        assert_eq!(
            positions.get("a.wav"),
            Some(&SampleMapLayoutPoint {
                x: 0.04,
                y: 0.06,
                cluster_id: Some(3),
            })
        );
        assert_eq!(
            positions.get("b.wav"),
            Some(&SampleMapLayoutPoint {
                x: 0.96,
                y: 0.94,
                cluster_id: Some(7),
            })
        );
    }

    #[test]
    fn sample_map_color_prefers_similarity_cluster_color() {
        let cluster_color = sample_map_color(SimilarityAspect::Spectrum, Some(0.5), Some(1));
        let aspect_color = sample_map_color(SimilarityAspect::Spectrum, Some(0.5), None);

        assert_ne!(cluster_color, aspect_color);
        assert_eq!(cluster_color, ui::Rgba8::new(57, 187, 245, 210));
    }

    #[test]
    fn selected_sample_map_position_uses_current_filtered_projection() {
        let root = tempfile::tempdir().expect("source root");
        let kick = root.path().join("kick.wav");
        std::fs::write(&kick, []).expect("write sample");
        let mut browser = FolderBrowserState::from_sample_sources(&[SampleSource::new(
            root.path().to_path_buf(),
        )]);
        let kick_id = kick.to_string_lossy().to_string();
        browser.select_file(kick_id.clone());
        let tags_by_file = HashMap::new();
        browser.prepare_sample_map_layout(&tags_by_file);

        let position = browser.selected_sample_map_position(SampleMapProjection {
            tags_by_file: &tags_by_file,
        });

        assert!(position.is_some());
        let projection = browser.sample_map_projection(SampleMapProjection {
            tags_by_file: &tags_by_file,
        });
        let selected = projection
            .iter()
            .find(|item| item.file_id == kick_id)
            .expect("selected map item");
        assert_eq!(position, Some((selected.x, selected.y)));
        assert!(selected.focused);
    }

    #[test]
    fn sample_map_projection_matches_filtered_browser_listing() {
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
        browser.prepare_sample_map_layout(&tags_by_file);

        let listing_ids = browser
            .browser_listing_snapshot(&tags_by_file)
            .ids()
            .to_vec();
        let map_ids = browser
            .sample_map_projection(SampleMapProjection {
                tags_by_file: &tags_by_file,
            })
            .into_iter()
            .map(|item| item.file_id)
            .collect::<Vec<_>>();

        assert_eq!(listing_ids, vec![kick_id, snare_id]);
        assert_eq!(
            map_ids, listing_ids,
            "sample map mode must project exactly the same filtered files as list mode"
        );
    }

    #[test]
    fn sample_map_projection_uses_full_filtered_listing_not_virtual_list_window() {
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
            .sample_map_projection(SampleMapProjection {
                tags_by_file: &tags_by_file,
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
            "sample map must include the full filtered listing, not only virtualized list rows"
        );
    }

    #[test]
    fn sample_map_projection_groups_by_enabled_similarity_aspects() {
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
            .sample_map_projection(SampleMapProjection {
                tags_by_file: &tags_by_file,
            })
            .into_iter()
            .find(|item| item.file_id == snare_id.as_str())
            .expect("snare map item")
            .color;

        let mut controls = browser.similarity_controls().clone();
        controls.set_aspect_enabled(SimilarityAspect::Timbre, false);
        browser.set_similarity_controls(controls);
        let spectrum_color = browser
            .sample_map_projection(SampleMapProjection {
                tags_by_file: &tags_by_file,
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
    fn sample_map_projection_marks_all_selected_list_items() {
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
            .sample_map_projection(SampleMapProjection {
                tags_by_file: &tags_by_file,
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
    fn sample_map_status_reports_incomplete_layout_coverage() {
        let status = SampleMapStatus {
            listed_count: 12,
            layout_count: 5,
            clustered_count: 2,
        };

        assert!(status.incomplete());
        assert_eq!(
            status.label(true),
            Some(String::from("Preparing similarity map 5 / 12"))
        );
        assert_eq!(
            status.label(false),
            Some(String::from("Similarity map 5 / 12"))
        );
    }

    #[test]
    fn complete_sample_map_status_stays_silent() {
        let status = SampleMapStatus {
            listed_count: 12,
            layout_count: 12,
            clustered_count: 8,
        };

        assert!(!status.incomplete());
        assert_eq!(status.label(true), None);
    }

    #[test]
    fn incomplete_sample_map_layout_requests_similarity_prep_once_per_signature() {
        let mut cache = SampleMapLayoutCache {
            signature: Some(42),
            listed_count: 2,
            points_by_file: HashMap::from([(
                String::from("a.wav"),
                SampleMapLayoutPoint {
                    x: 0.2,
                    y: 0.3,
                    cluster_id: None,
                },
            )]),
            auto_prep_requested_signature: None,
        };

        assert!(cache.needs_similarity_prep());

        cache.auto_prep_requested_signature = Some(42);

        assert!(!cache.needs_similarity_prep());
    }

    #[test]
    fn complete_or_empty_sample_map_layout_does_not_request_similarity_prep() {
        let complete = SampleMapLayoutCache {
            signature: Some(7),
            listed_count: 1,
            points_by_file: HashMap::from([(
                String::from("a.wav"),
                SampleMapLayoutPoint {
                    x: 0.2,
                    y: 0.3,
                    cluster_id: None,
                },
            )]),
            auto_prep_requested_signature: None,
        };
        let empty_listing = SampleMapLayoutCache {
            signature: Some(8),
            listed_count: 0,
            points_by_file: HashMap::new(),
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
}
