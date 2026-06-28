use super::super::{UmapPoint, UmapPointQuery};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

pub(crate) fn load_umap_points(
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

pub(crate) fn load_umap_point_for_sample(
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
        .map_err(|err| format!("Prepare map-layout point query failed: {err}"))?;
    stmt.query_row(params![model_id, umap_version, sample_id], |row| {
        let x: f32 = row.get(0)?;
        let y: f32 = row.get(1)?;
        Ok((x, y))
    })
    .optional()
    .map_err(|err| format!("Query map-layout point failed: {err}"))
}

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
