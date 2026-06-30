use std::collections::HashMap;

use rusqlite::params_from_iter;
use wavecrate_analysis::similarity::SIMILARITY_MODEL_ID;

use crate::sample_sources::{SampleSource, SourceDatabase, SourceDatabaseConnectionRole};

/// UMAP artifact version used by the native starmap layout.
pub const STARMAP_LAYOUT_UMAP_VERSION: &str = "v1";

/// Background request for loading starmap layout points from source databases.
#[derive(Clone, Debug)]
pub struct StarmapLayoutLoadRequest {
    /// Native cache signature used to ignore stale background results.
    pub signature: u64,
    /// Per-source layout lookups to execute.
    pub sources: Vec<StarmapSourceLayoutRequest>,
}

impl StarmapLayoutLoadRequest {
    /// Return whether this request contains no sample lookups.
    pub fn is_empty(&self) -> bool {
        self.sources.iter().all(|source| source.samples.is_empty())
    }
}

/// Source-specific starmap layout lookup request.
#[derive(Clone, Debug)]
pub struct StarmapSourceLayoutRequest {
    /// Sample source whose metadata database contains the layout artifacts.
    pub source: SampleSource,
    /// Samples to load from this source database.
    pub samples: Vec<StarmapLayoutSample>,
}

/// A starmap layout lookup for one file/sample id pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StarmapLayoutSample {
    /// Absolute file id/path used by the native browser.
    pub file_id: String,
    /// Stable analysis sample id stored in the source database.
    pub sample_id: String,
}

/// Normalized starmap position plus optional cluster id for one browser file.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarmapLayoutPoint {
    /// Normalized x coordinate in the starmap domain.
    pub x: f32,
    /// Normalized y coordinate in the starmap domain.
    pub y: f32,
    /// Optional HDBSCAN cluster id for color grouping.
    pub cluster_id: Option<i32>,
}

/// Result of a background starmap layout load.
#[derive(Clone, Debug, PartialEq)]
pub struct StarmapLayoutLoadResult {
    /// Native cache signature used to ignore stale background results.
    pub signature: u64,
    /// Loaded layout points keyed by native browser file id.
    pub result: Result<HashMap<String, StarmapLayoutPoint>, String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RawStarmapLayoutPoint {
    x: f32,
    y: f32,
    cluster_id: Option<i32>,
}

/// Load and normalize starmap layout points from source metadata databases.
pub fn load_starmap_layout(request: StarmapLayoutLoadRequest) -> StarmapLayoutLoadResult {
    let signature = request.signature;
    StarmapLayoutLoadResult {
        signature,
        result: load_starmap_layout_inner(request),
    }
}

fn load_starmap_layout_inner(
    request: StarmapLayoutLoadRequest,
) -> Result<HashMap<String, StarmapLayoutPoint>, String> {
    let mut raw_points = HashMap::new();
    for source in request.sources {
        load_source_layout_positions(&source, &mut raw_points)?;
    }
    Ok(normalized_layout_points(raw_points))
}

fn load_source_layout_positions(
    request: &StarmapSourceLayoutRequest,
    positions: &mut HashMap<String, RawStarmapLayoutPoint>,
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
        params.push(STARMAP_LAYOUT_UMAP_VERSION.to_string());
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
                RawStarmapLayoutPoint { x, y, cluster_id },
            );
        }
    }
    Ok(())
}

fn normalized_layout_points(
    raw_points: HashMap<String, RawStarmapLayoutPoint>,
) -> HashMap<String, StarmapLayoutPoint> {
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
                StarmapLayoutPoint {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_layout_positions_fill_map_domain() {
        let positions = normalized_layout_points(HashMap::from([
            (
                String::from("a.wav"),
                RawStarmapLayoutPoint {
                    x: -1.0,
                    y: 2.0,
                    cluster_id: Some(3),
                },
            ),
            (
                String::from("b.wav"),
                RawStarmapLayoutPoint {
                    x: 1.0,
                    y: 6.0,
                    cluster_id: Some(7),
                },
            ),
        ]));

        assert_eq!(
            positions.get("a.wav"),
            Some(&StarmapLayoutPoint {
                x: 0.04,
                y: 0.06,
                cluster_id: Some(3),
            })
        );
        assert_eq!(
            positions.get("b.wav"),
            Some(&StarmapLayoutPoint {
                x: 0.96,
                y: 0.94,
                cluster_id: Some(7),
            })
        );
    }
}
