use crate::app::state::MapClusterCentroid;
use crate::sample_sources::SourceId;
use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use std::collections::HashMap;

pub(crate) fn load_umap_cluster_centroids(
    conn: &mut Connection,
    model_id: &str,
    umap_version: &str,
    cluster_method: &str,
    cluster_umap_version: &str,
    source_id: Option<&SourceId>,
) -> Result<HashMap<i32, MapClusterCentroid>, String> {
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
                MapClusterCentroid {
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
