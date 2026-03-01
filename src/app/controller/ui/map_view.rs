use super::*;
use crate::app::controller::library::analysis_jobs;
use crate::app::state::SampleBrowserTab;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use std::collections::HashMap;

pub(crate) struct UmapBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

pub(crate) struct UmapPoint {
    pub sample_id: String,
    pub x: f32,
    pub y: f32,
    pub cluster_id: Option<i32>,
}

/// Query payload for loading visible UMAP points and optional cluster metadata.
pub(crate) struct UmapPointQuery<'a> {
    pub model_id: &'a str,
    pub umap_version: &'a str,
    pub cluster_method: &'a str,
    pub cluster_umap_version: &'a str,
    pub source_id: Option<&'a SourceId>,
    pub bounds: crate::app::state::MapQueryBounds,
    pub limit: usize,
}

impl AppController {
    /// Switch between browser list/map tabs and keep map visibility in sync.
    pub fn set_browser_tab(&mut self, map: bool) {
        self.ui.browser.active_tab = if map {
            SampleBrowserTab::Map
        } else {
            SampleBrowserTab::List
        };
        self.ui.map.open = map;
    }

    /// Stage map focus/hover ids before resolving sample focus and preview.
    pub fn stage_map_sample_focus(&mut self, sample_id: &str) {
        let sample_id = sample_id.to_string();
        let selection_changed = self.ui.map.selected_sample_id.as_ref() != Some(&sample_id);
        let hover_changed = self.ui.map.hovered_sample_id.as_ref() != Some(&sample_id);
        self.ui.map.selected_sample_id = Some(sample_id.clone());
        self.ui.map.hovered_sample_id = Some(sample_id.clone());
        self.ui.map.paint_hover_active_id = Some(sample_id);
        if selection_changed {
            self.mark_map_selection_projection_revision_dirty();
        }
        if hover_changed {
            self.mark_map_hover_projection_revision_dirty();
        }
    }

    /// Focus a map sample, stage hover/selection ids, queue load preview, and start playback.
    pub fn focus_map_sample_and_preview(&mut self, sample_id: &str) {
        self.set_browser_tab(true);
        self.stage_map_sample_focus(sample_id);
        if let Err(err) = self.focus_sample_from_map(sample_id) {
            self.set_error_status(format!("Map focus failed: {err}"));
            return;
        }
        if let Err(err) = self.play_audio(false, None) {
            self.set_error_status(format!("Playback failed: {err}"));
        }
    }

    /// Open the map view panel.
    pub fn open_map(&mut self) {
        self.ui.map.open = true;
    }

    /// Enqueue a UMAP layout build for the selected source.
    pub fn build_umap_layout(&mut self, model_id: &str, umap_version: &str) {
        if self.runtime.jobs.umap_build_in_progress() {
            self.set_status_message(StatusMessage::TsneBuildAlreadyRunning);
            return;
        }
        let Some(source_id) = self.current_source().map(|source| source.id) else {
            self.set_status_message(StatusMessage::SelectSourceFirst {
                tone: StatusTone::Warning,
            });
            return;
        };
        self.runtime
            .jobs
            .begin_umap_build(super::jobs::UmapBuildJob {
                model_id: model_id.to_string(),
                umap_version: umap_version.to_string(),
                source_id,
            });
        self.set_status_message(StatusMessage::BuildingTsneLayout);
    }

    /// Enqueue cluster generation for the current UMAP layout.
    pub fn build_umap_clusters(&mut self, model_id: &str, umap_version: &str) {
        if self.runtime.jobs.umap_cluster_build_in_progress() {
            self.set_status_message(StatusMessage::ClusterBuildAlreadyRunning);
            return;
        }
        let source_id = self.current_source().map(|source| source.id);
        self.runtime
            .jobs
            .begin_umap_cluster_build(super::jobs::UmapClusterBuildJob {
                model_id: model_id.to_string(),
                umap_version: umap_version.to_string(),
                source_id,
            });
        self.set_status_message(StatusMessage::BuildingClusters);
    }

    pub(crate) fn umap_bounds(
        &mut self,
        model_id: &str,
        umap_version: &str,
        source_id: Option<&SourceId>,
    ) -> Result<Option<UmapBounds>, String> {
        let conn = open_source_db(self, source_id)?;
        load_umap_bounds(conn, model_id, umap_version, source_id)
    }

    pub(crate) fn umap_points_in_bounds(
        &mut self,
        query: UmapPointQuery<'_>,
    ) -> Result<Vec<UmapPoint>, String> {
        let conn = open_source_db(self, query.source_id)?;
        load_umap_points(conn, &query)
    }

    /// Lookup a UMAP point for a specific sample id.
    pub fn umap_point_for_sample(
        &mut self,
        model_id: &str,
        umap_version: &str,
        sample_id: &str,
    ) -> Result<Option<(f32, f32)>, String> {
        let (source_id, _relative) = analysis_jobs::parse_sample_id(sample_id)?;
        let source_id = SourceId::from_string(source_id);
        let conn = open_source_db(self, Some(&source_id))?;
        load_umap_point_for_sample(conn, model_id, umap_version, sample_id)
    }

    /// Load cluster centroids for the requested UMAP layout.
    pub fn umap_cluster_centroids(
        &mut self,
        model_id: &str,
        umap_version: &str,
        cluster_method: &str,
        cluster_umap_version: &str,
        source_id: Option<&SourceId>,
    ) -> Result<HashMap<i32, crate::app::state::MapClusterCentroid>, String> {
        let conn = open_source_db(self, source_id)?;
        load_umap_cluster_centroids(
            conn,
            model_id,
            umap_version,
            cluster_method,
            cluster_umap_version,
            source_id,
        )
    }
}

pub(crate) fn run_umap_build(
    model_id: &str,
    umap_version: &str,
    source_id: &SourceId,
) -> Result<(), String> {
    let mut conn = open_source_db_for_id(source_id)?;
    crate::analysis::umap::build_umap_layout(&mut conn, model_id, umap_version, 0, 0.95)?;
    Ok(())
}

pub(crate) fn run_umap_cluster_build(
    model_id: &str,
    umap_version: &str,
    source_id: Option<&SourceId>,
) -> Result<crate::analysis::hdbscan::HdbscanStats, String> {
    let Some(source_id) = source_id else {
        return Err("Missing source for cluster build".to_string());
    };
    let mut conn = open_source_db_for_id(source_id)?;
    let sample_id_prefix = Some(format!("{}::%", source_id.as_str()));
    crate::analysis::hdbscan::build_hdbscan_clusters_for_sample_id_prefix(
        &mut conn,
        model_id,
        crate::analysis::hdbscan::HdbscanMethod::Umap,
        Some(umap_version),
        sample_id_prefix.as_deref(),
        crate::analysis::hdbscan::HdbscanConfig {
            min_cluster_size:
                crate::app::controller::library::similarity_prep::DEFAULT_CLUSTER_MIN_SIZE,
            min_samples: None,
            allow_single_cluster: false,
        },
    )
}

/// Return a cached per-source map-query connection, opening it on first use.
fn open_source_db<'a>(
    controller: &'a mut AppController,
    source_id: Option<&SourceId>,
) -> Result<&'a mut Connection, String> {
    let source_id = source_id
        .ok_or_else(|| "No source selected".to_string())?
        .clone();
    let source_root = controller
        .library
        .sources
        .iter()
        .find(|source| source.id == source_id)
        .map(|source| source.root.clone())
        .ok_or_else(|| "Source not found".to_string())?;
    if !controller
        .runtime
        .map_query_connections
        .contains_key(&source_id)
    {
        let conn = analysis_jobs::open_source_db(&source_root)?;
        controller
            .runtime
            .map_query_connections
            .insert(source_id.clone(), conn);
    }
    controller
        .runtime
        .map_query_connections
        .get_mut(&source_id)
        .ok_or_else(|| "Map query connection missing after open".to_string())
}

fn open_source_db_for_id(source_id: &SourceId) -> Result<Connection, String> {
    let state = crate::sample_sources::library::load().map_err(|err| err.to_string())?;
    let source = state
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found".to_string())?;
    analysis_jobs::open_source_db(&source.root)
}

fn load_umap_bounds(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    source_id: Option<&SourceId>,
) -> Result<Option<UmapBounds>, String> {
    let row = if let Some(source_id) = source_id {
        let prefix = format!("{}::%", source_id.as_str());
        let mut stmt = conn
            .prepare_cached(
                "SELECT MIN(x), MAX(x), MIN(y), MAX(y)
                 FROM layout_umap
                 WHERE model_id = ?1 AND umap_version = ?2
                   AND sample_id LIKE ?3",
            )
            .map_err(|err| format!("Prepare t-SNE bounds query failed: {err}"))?;
        stmt.query_row(params![model_id, umap_version, prefix], |row| {
            let min_x: Option<f32> = row.get(0)?;
            let max_x: Option<f32> = row.get(1)?;
            let min_y: Option<f32> = row.get(2)?;
            let max_y: Option<f32> = row.get(3)?;
            Ok((min_x, max_x, min_y, max_y))
        })
        .optional()
        .map_err(|err| format!("Query t-SNE bounds failed: {err}"))?
    } else {
        let mut stmt = conn
            .prepare_cached(
                "SELECT MIN(x), MAX(x), MIN(y), MAX(y)
                 FROM layout_umap
                 WHERE model_id = ?1 AND umap_version = ?2",
            )
            .map_err(|err| format!("Prepare t-SNE bounds query failed: {err}"))?;
        stmt.query_row(params![model_id, umap_version], |row| {
            let min_x: Option<f32> = row.get(0)?;
            let max_x: Option<f32> = row.get(1)?;
            let min_y: Option<f32> = row.get(2)?;
            let max_y: Option<f32> = row.get(3)?;
            Ok((min_x, max_x, min_y, max_y))
        })
        .optional()
        .map_err(|err| format!("Query t-SNE bounds failed: {err}"))?
    };
    let Some((min_x, max_x, min_y, max_y)) = row else {
        return Ok(None);
    };
    match (min_x, max_x, min_y, max_y) {
        (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) => Ok(Some(UmapBounds {
            min_x,
            max_x,
            min_y,
            max_y,
        })),
        _ => Ok(None),
    }
}

fn load_umap_points(
    conn: &mut Connection,
    query: &UmapPointQuery<'_>,
) -> Result<Vec<UmapPoint>, String> {
    let (sql, params) = if let Some(source_id) = query.source_id {
        let prefix = format!("{}::%", source_id.as_str());
        (
            "SELECT layout_umap.sample_id, layout_umap.x, layout_umap.y, hdbscan_clusters.cluster_id
             FROM layout_umap
             LEFT JOIN hdbscan_clusters
                ON layout_umap.sample_id = hdbscan_clusters.sample_id
               AND hdbscan_clusters.model_id = ?1
               AND hdbscan_clusters.method = ?3
               AND hdbscan_clusters.umap_version = ?4
             WHERE layout_umap.model_id = ?1 AND layout_umap.umap_version = ?2
               AND layout_umap.sample_id LIKE ?5
               AND layout_umap.x >= ?6 AND layout_umap.x <= ?7
               AND layout_umap.y >= ?8 AND layout_umap.y <= ?9
             ORDER BY layout_umap.sample_id ASC
             LIMIT ?10",
            vec![
                Value::Text(query.model_id.to_string()),
                Value::Text(query.umap_version.to_string()),
                Value::Text(query.cluster_method.to_string()),
                Value::Text(query.cluster_umap_version.to_string()),
                Value::Text(prefix),
                Value::Real(query.bounds.min_x as f64),
                Value::Real(query.bounds.max_x as f64),
                Value::Real(query.bounds.min_y as f64),
                Value::Real(query.bounds.max_y as f64),
                Value::Integer(query.limit as i64),
            ],
        )
    } else {
        (
            "SELECT layout_umap.sample_id, layout_umap.x, layout_umap.y, hdbscan_clusters.cluster_id
             FROM layout_umap
             LEFT JOIN hdbscan_clusters
                ON layout_umap.sample_id = hdbscan_clusters.sample_id
               AND hdbscan_clusters.model_id = ?1
               AND hdbscan_clusters.method = ?3
               AND hdbscan_clusters.umap_version = ?4
             WHERE layout_umap.model_id = ?1 AND layout_umap.umap_version = ?2
               AND layout_umap.x >= ?5 AND layout_umap.x <= ?6
               AND layout_umap.y >= ?7 AND layout_umap.y <= ?8
             ORDER BY layout_umap.sample_id ASC
             LIMIT ?9",
            vec![
                Value::Text(query.model_id.to_string()),
                Value::Text(query.umap_version.to_string()),
                Value::Text(query.cluster_method.to_string()),
                Value::Text(query.cluster_umap_version.to_string()),
                Value::Real(query.bounds.min_x as f64),
                Value::Real(query.bounds.max_x as f64),
                Value::Real(query.bounds.min_y as f64),
                Value::Real(query.bounds.max_y as f64),
                Value::Integer(query.limit as i64),
            ],
        )
    };
    let mut stmt = conn
        .prepare_cached(sql)
        .map_err(|err| format!("Prepare layout query failed: {err}"))?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            let cluster_id: Option<i64> = row.get(3)?;
            Ok(UmapPoint {
                sample_id: row.get(0)?,
                x: row.get::<_, f32>(1)?,
                y: row.get::<_, f32>(2)?,
                cluster_id: cluster_id.map(|id| id as i32),
            })
        })
        .map_err(|err| format!("Query layout points failed: {err}"))?;
    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|err| format!("Read layout row failed: {err}"))?);
    }
    Ok(points)
}

fn load_umap_point_for_sample(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    sample_id: &str,
) -> Result<Option<(f32, f32)>, String> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT x, y
             FROM layout_umap
             WHERE model_id = ?1 AND umap_version = ?2 AND sample_id = ?3",
        )
        .map_err(|err| format!("Prepare t-SNE point query failed: {err}"))?;
    stmt.query_row(params![model_id, umap_version, sample_id], |row| {
        let x: f32 = row.get(0)?;
        let y: f32 = row.get(1)?;
        Ok((x, y))
    })
    .optional()
    .map_err(|err| format!("Query t-SNE point failed: {err}"))
}

fn load_umap_cluster_centroids(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    cluster_method: &str,
    cluster_umap_version: &str,
    source_id: Option<&SourceId>,
) -> Result<HashMap<i32, crate::app::state::MapClusterCentroid>, String> {
    let (sql, params) = if let Some(source_id) = source_id {
        let prefix = format!("{}::%", source_id.as_str());
        (
            "SELECT hdbscan_clusters.cluster_id, AVG(layout_umap.x), AVG(layout_umap.y), COUNT(*)
             FROM layout_umap
             JOIN hdbscan_clusters
               ON layout_umap.sample_id = hdbscan_clusters.sample_id
              AND hdbscan_clusters.model_id = ?1
              AND hdbscan_clusters.method = ?3
              AND hdbscan_clusters.umap_version = ?4
             WHERE layout_umap.model_id = ?1 AND layout_umap.umap_version = ?2
               AND layout_umap.sample_id LIKE ?5
             GROUP BY hdbscan_clusters.cluster_id",
            vec![
                Value::Text(model_id.to_string()),
                Value::Text(umap_version.to_string()),
                Value::Text(cluster_method.to_string()),
                Value::Text(cluster_umap_version.to_string()),
                Value::Text(prefix),
            ],
        )
    } else {
        (
            "SELECT hdbscan_clusters.cluster_id, AVG(layout_umap.x), AVG(layout_umap.y), COUNT(*)
             FROM layout_umap
             JOIN hdbscan_clusters
               ON layout_umap.sample_id = hdbscan_clusters.sample_id
              AND hdbscan_clusters.model_id = ?1
              AND hdbscan_clusters.method = ?3
              AND hdbscan_clusters.umap_version = ?4
             WHERE layout_umap.model_id = ?1 AND layout_umap.umap_version = ?2
             GROUP BY hdbscan_clusters.cluster_id",
            vec![
                Value::Text(model_id.to_string()),
                Value::Text(umap_version.to_string()),
                Value::Text(cluster_method.to_string()),
                Value::Text(cluster_umap_version.to_string()),
            ],
        )
    };

    let mut stmt = conn
        .prepare_cached(sql)
        .map_err(|err| format!("Prepare centroid query failed: {err}"))?;
    let rows = stmt
        .query_map(params_from_iter(params), |row| {
            let cluster_id: i64 = row.get(0)?;
            let x: f64 = row.get(1)?;
            let y: f64 = row.get(2)?;
            let count: i64 = row.get(3)?;
            Ok((
                cluster_id as i32,
                crate::app::state::MapClusterCentroid {
                    x: x as f32,
                    y: y as f32,
                    count: count as usize,
                },
            ))
        })
        .map_err(|err| format!("Query centroids failed: {err}"))?;

    let mut centroids = HashMap::new();
    for row in rows {
        let (cluster_id, centroid) =
            row.map_err(|err| format!("Read centroid row failed: {err}"))?;
        centroids.insert(cluster_id, centroid);
    }
    Ok(centroids)
}
