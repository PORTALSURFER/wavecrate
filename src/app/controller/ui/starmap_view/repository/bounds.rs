use super::super::UmapBounds;
use crate::sample_sources::SourceId;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params_from_iter};

pub(crate) fn load_umap_bounds(
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
        .map_err(|err| format!("Prepare map-layout bounds query failed: {err}"))?;
    stmt.query_row(params_from_iter(params), |row| {
        let min_x: Option<f32> = row.get(0)?;
        let max_x: Option<f32> = row.get(1)?;
        let min_y: Option<f32> = row.get(2)?;
        let max_y: Option<f32> = row.get(3)?;
        Ok((min_x, max_x, min_y, max_y))
    })
    .optional()
    .map_err(|err| format!("Query map-layout bounds failed: {err}"))
}

fn bounds_row_has_values(bounds: (Option<f32>, Option<f32>, Option<f32>, Option<f32>)) -> bool {
    matches!(bounds, (Some(_), Some(_), Some(_), Some(_)))
}
