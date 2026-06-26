use std::collections::HashMap;
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

#[derive(Clone, Debug, Default)]
pub(super) struct SampleMapLayoutCache {
    signature: Option<u64>,
    pub(super) positions_by_file: HashMap<String, (f32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SampleMapItem {
    pub(in crate::native_app) file_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) x: f32,
    pub(in crate::native_app) y: f32,
    pub(in crate::native_app) color: ui::Rgba8,
    pub(in crate::native_app) selected: bool,
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
            positions_by_file: positions,
        };
    }

    pub(in crate::native_app) fn sample_map_projection(
        &self,
        projection: SampleMapProjection<'_>,
    ) -> Vec<SampleMapItem> {
        let snapshot = self.browser_listing_snapshot(projection.tags_by_file);
        snapshot
            .rows()
            .iter()
            .map(|file| {
                let aspects = self.similarity_aspect_display_strengths_for_file(&file.id);
                let strength = self.similarity_display_strength_for_file(&file.id);
                let group = strongest_enabled_aspect(&aspects, self.similarity_controls());
                let layout_position = self
                    .sample_list
                    .sample_map_layout
                    .positions_by_file
                    .get(&file.id)
                    .copied();
                let (x, y) = sample_map_position(&file.id, group, strength, layout_position);
                SampleMapItem {
                    file_id: file.id.clone(),
                    label: file.stem.clone(),
                    x,
                    y,
                    color: sample_map_color(group, strength),
                    selected: self.is_file_selected(&file.id),
                    similarity_anchor: self.file_is_similarity_anchor(&file.id),
                    missing: file.is_missing(),
                }
            })
            .collect()
    }
}

fn load_sample_map_layout_positions(
    browser: &FolderBrowserState,
    files: &[&FileEntry],
) -> Result<HashMap<String, (f32, f32)>, String> {
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

    let mut raw_positions = HashMap::new();
    for request in by_source.values() {
        load_source_layout_positions(request, &mut raw_positions)?;
    }
    Ok(normalized_layout_positions(raw_positions))
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
    positions: &mut HashMap<String, (f32, f32)>,
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
            "SELECT sample_id, x, y FROM layout_umap \
             WHERE model_id = ?1 AND umap_version = ?2 AND sample_id IN (",
        );
        query.push_str(
            &std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(","),
        );
        query.push(')');

        let mut params = Vec::with_capacity(chunk.len() + 2);
        params.push(SIMILARITY_MODEL_ID.to_string());
        params.push(NATIVE_SIMILARITY_UMAP_VERSION.to_string());
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
                ))
            })
            .map_err(|err| format!("Query map layout failed: {err}"))?;
        for row in rows {
            let (sample_id, x, y) =
                row.map_err(|err| format!("Decode map layout row failed: {err}"))?;
            let Some(file_id) = file_by_sample_id.get(sample_id.as_str()) else {
                continue;
            };
            positions.insert((*file_id).to_string(), (x, y));
        }
    }
    Ok(())
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

fn normalized_layout_positions(
    raw_positions: HashMap<String, (f32, f32)>,
) -> HashMap<String, (f32, f32)> {
    if raw_positions.is_empty() {
        return HashMap::new();
    }
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for (x, y) in raw_positions.values().copied() {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    raw_positions
        .into_iter()
        .map(|(file_id, (x, y))| {
            (
                file_id,
                (
                    normalize_layout_axis(x, min_x, max_x, 0.04, 0.96),
                    normalize_layout_axis(y, min_y, max_y, 0.06, 0.94),
                ),
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

fn sample_map_color(group: SimilarityAspect, strength: Option<f32>) -> ui::Rgba8 {
    let alpha = (150.0 + strength.unwrap_or(0.35).clamp(0.0, 1.0) * 90.0) as u8;
    match group {
        SimilarityAspect::Overall => ui::Rgba8::new(122, 226, 96, alpha),
        SimilarityAspect::Spectrum => ui::Rgba8::new(239, 216, 66, alpha),
        SimilarityAspect::Timbre => ui::Rgba8::new(255, 142, 56, alpha),
        SimilarityAspect::Pitch => ui::Rgba8::new(255, 55, 96, alpha),
        SimilarityAspect::Amplitude => ui::Rgba8::new(57, 187, 245, alpha),
    }
}

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
        let positions = normalized_layout_positions(HashMap::from([
            (String::from("a.wav"), (-1.0, 2.0)),
            (String::from("b.wav"), (1.0, 6.0)),
        ]));

        assert_eq!(positions.get("a.wav"), Some(&(0.04, 0.06)));
        assert_eq!(positions.get("b.wav"), Some(&(0.96, 0.94)));
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
