use super::{UmapBounds, UmapPoint, UmapPointQuery};
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::SourceId;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use std::collections::HashMap;

pub(super) fn open_source_db_for_id(source_id: &SourceId) -> Result<Connection, String> {
    let state = crate::sample_sources::library::load().map_err(|err| err.to_string())?;
    let source = state
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found".to_string())?;
    analysis_jobs::open_source_db(&source.root)
}

pub(super) fn load_umap_bounds(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    source_id: Option<&SourceId>,
) -> Result<Option<UmapBounds>, String> {
    let row = if let Some(source_id) = source_id {
        let prefix = format!("{}::%", source_id.as_str());
        let filtered = query_umap_bounds(conn, model_id, umap_version, Some(prefix.as_str()))?;
        if filtered.is_some_and(bounds_row_has_values) {
            filtered
        } else {
            query_umap_bounds(conn, model_id, umap_version, None)?
        }
    } else {
        query_umap_bounds(conn, model_id, umap_version, None)?
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

/// Query aggregate UMAP bounds with an optional sample-id prefix filter.
fn query_umap_bounds(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    sample_prefix: Option<&str>,
) -> Result<Option<(Option<f32>, Option<f32>, Option<f32>, Option<f32>)>, String> {
    let (sql, params): (&str, Vec<Value>) = if let Some(prefix) = sample_prefix {
        (
            "SELECT MIN(x), MAX(x), MIN(y), MAX(y)
             FROM layout_umap
             WHERE model_id = ?1 AND umap_version = ?2
               AND sample_id LIKE ?3",
            vec![
                Value::Text(model_id.to_string()),
                Value::Text(umap_version.to_string()),
                Value::Text(prefix.to_string()),
            ],
        )
    } else {
        (
            "SELECT MIN(x), MAX(x), MIN(y), MAX(y)
             FROM layout_umap
             WHERE model_id = ?1 AND umap_version = ?2",
            vec![
                Value::Text(model_id.to_string()),
                Value::Text(umap_version.to_string()),
            ],
        )
    };
    let mut stmt = conn
        .prepare_cached(sql)
        .map_err(|err| format!("Prepare t-SNE bounds query failed: {err}"))?;
    stmt.query_row(params_from_iter(params), |row| {
        let min_x: Option<f32> = row.get(0)?;
        let max_x: Option<f32> = row.get(1)?;
        let min_y: Option<f32> = row.get(2)?;
        let max_y: Option<f32> = row.get(3)?;
        Ok((min_x, max_x, min_y, max_y))
    })
    .optional()
    .map_err(|err| format!("Query t-SNE bounds failed: {err}"))
}

/// Return whether an aggregate-bounds row contains concrete coordinate values.
fn bounds_row_has_values(bounds: (Option<f32>, Option<f32>, Option<f32>, Option<f32>)) -> bool {
    matches!(bounds, (Some(_), Some(_), Some(_), Some(_)))
}

pub(super) fn load_umap_points(
    conn: &mut Connection,
    query: &UmapPointQuery<'_>,
) -> Result<Vec<UmapPoint>, String> {
    let points = if let Some(source_id) = query.source_id {
        let prefix = format!("{}::%", source_id.as_str());
        let filtered = query_umap_points(conn, query, Some(prefix.as_str()))?;
        if filtered.is_empty() {
            query_umap_points(conn, query, None)?
        } else {
            filtered
        }
    } else {
        query_umap_points(conn, query, None)?
    };
    Ok(points)
}

/// Query rendered UMAP points with an optional sample-id prefix filter.
fn query_umap_points(
    conn: &mut Connection,
    query: &UmapPointQuery<'_>,
    sample_prefix: Option<&str>,
) -> Result<Vec<UmapPoint>, String> {
    let (sql, params) = if let Some(prefix) = sample_prefix {
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
                Value::Text(prefix.to_string()),
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

pub(super) fn load_umap_point_for_sample(
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

pub(super) fn load_umap_cluster_centroids(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory sqlite should open");
        conn.execute_batch(
            "CREATE TABLE layout_umap (
                 sample_id TEXT PRIMARY KEY,
                 model_id TEXT NOT NULL,
                 umap_version TEXT NOT NULL,
                 x REAL NOT NULL,
                 y REAL NOT NULL
             );
             CREATE TABLE hdbscan_clusters (
                 sample_id TEXT NOT NULL,
                 model_id TEXT NOT NULL,
                 method TEXT NOT NULL,
                 umap_version TEXT,
                 cluster_id INTEGER NOT NULL
             );",
        )
        .expect("test schema should apply");
        conn
    }

    fn seed_layout(conn: &Connection) {
        conn.execute(
            "INSERT INTO layout_umap (sample_id, model_id, umap_version, x, y)
             VALUES ('source-a::kick.wav', 'model', 'umap-v1', 1.0, 2.0)",
            [],
        )
        .expect("seed source-a kick");
        conn.execute(
            "INSERT INTO layout_umap (sample_id, model_id, umap_version, x, y)
             VALUES ('source-a::snare.wav', 'model', 'umap-v1', 3.0, 4.0)",
            [],
        )
        .expect("seed source-a snare");
        conn.execute(
            "INSERT INTO layout_umap (sample_id, model_id, umap_version, x, y)
             VALUES ('source-b::hat.wav', 'model', 'umap-v1', 9.0, 8.0)",
            [],
        )
        .expect("seed source-b hat");
        conn.execute(
            "INSERT INTO hdbscan_clusters (sample_id, model_id, method, umap_version, cluster_id)
             VALUES ('source-a::kick.wav', 'model', 'hdbscan', 'umap-v1', 7)",
            [],
        )
        .expect("seed source-a cluster");
        conn.execute(
            "INSERT INTO hdbscan_clusters (sample_id, model_id, method, umap_version, cluster_id)
             VALUES ('source-a::snare.wav', 'model', 'hdbscan', 'umap-v1', 7)",
            [],
        )
        .expect("seed second source-a cluster");
    }

    #[test]
    fn load_umap_bounds_filters_by_source_prefix() {
        let mut conn = test_connection();
        seed_layout(&conn);

        let bounds = load_umap_bounds(
            &mut conn,
            "model",
            "umap-v1",
            Some(&SourceId::from_string("source-a")),
        )
        .expect("bounds query should succeed")
        .expect("source-a bounds should exist");

        assert_eq!(bounds.min_x, 1.0);
        assert_eq!(bounds.max_x, 3.0);
        assert_eq!(bounds.min_y, 2.0);
        assert_eq!(bounds.max_y, 4.0);
    }

    #[test]
    fn load_umap_bounds_falls_back_when_source_prefix_misses() {
        let mut conn = test_connection();
        seed_layout(&conn);

        let bounds = load_umap_bounds(
            &mut conn,
            "model",
            "umap-v1",
            Some(&SourceId::from_string("source-z")),
        )
        .expect("bounds query should succeed")
        .expect("fallback bounds should exist");

        assert_eq!(bounds.min_x, 1.0);
        assert_eq!(bounds.max_x, 9.0);
        assert_eq!(bounds.min_y, 2.0);
        assert_eq!(bounds.max_y, 8.0);
    }

    #[test]
    fn load_umap_points_joins_clusters_and_applies_bounds() {
        let mut conn = test_connection();
        seed_layout(&conn);

        let query = UmapPointQuery {
            model_id: "model",
            umap_version: "umap-v1",
            cluster_method: "hdbscan",
            cluster_umap_version: "umap-v1",
            source_id: Some(&SourceId::from_string("source-a")),
            bounds: crate::app::state::MapQueryBounds {
                min_x: 0.0,
                max_x: 5.0,
                min_y: 0.0,
                max_y: 5.0,
            },
            limit: 10,
        };

        let points = load_umap_points(&mut conn, &query).expect("points query should succeed");

        assert_eq!(points.len(), 2);
        assert_eq!(points[0].sample_id, "source-a::kick.wav");
        assert_eq!(points[0].cluster_id, Some(7));
        assert_eq!(points[1].sample_id, "source-a::snare.wav");
        assert_eq!(points[1].cluster_id, Some(7));
    }

    #[test]
    fn load_umap_points_fall_back_when_source_prefix_misses() {
        let mut conn = test_connection();
        seed_layout(&conn);

        let query = UmapPointQuery {
            model_id: "model",
            umap_version: "umap-v1",
            cluster_method: "hdbscan",
            cluster_umap_version: "umap-v1",
            source_id: Some(&SourceId::from_string("source-z")),
            bounds: crate::app::state::MapQueryBounds {
                min_x: 0.0,
                max_x: 10.0,
                min_y: 0.0,
                max_y: 10.0,
            },
            limit: 10,
        };

        let points = load_umap_points(&mut conn, &query).expect("points query should succeed");

        assert_eq!(points.len(), 3);
        assert_eq!(points[0].sample_id, "source-a::kick.wav");
        assert_eq!(points[2].sample_id, "source-b::hat.wav");
    }

    #[test]
    fn load_umap_cluster_centroids_groups_filtered_rows() {
        let mut conn = test_connection();
        seed_layout(&conn);

        let centroids = load_umap_cluster_centroids(
            &mut conn,
            "model",
            "umap-v1",
            "hdbscan",
            "umap-v1",
            Some(&SourceId::from_string("source-a")),
        )
        .expect("centroid query should succeed");

        let centroid = centroids.get(&7).expect("cluster centroid should exist");
        assert_eq!(centroid.x, 2.0);
        assert_eq!(centroid.y, 3.0);
        assert_eq!(centroid.count, 2);
    }
}
