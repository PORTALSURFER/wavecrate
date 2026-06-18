use super::batch::{SQLITE_IN_BATCH_SIZE, aspect_descriptor_query_values, placeholder_list};
use rusqlite::params_from_iter;
use std::collections::HashMap;

/// Load persisted aspect descriptors for a candidate set in batched queries.
pub(crate) fn load_aspect_descriptors_for_samples(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
) -> Result<HashMap<String, wavecrate_analysis::aspects::AspectDescriptorSet>, String> {
    if sample_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut descriptors = HashMap::with_capacity(sample_ids.len());
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        load_aspect_descriptor_batch(conn, batch, &mut descriptors)?;
    }
    Ok(descriptors)
}

fn load_aspect_descriptor_batch(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    descriptors: &mut HashMap<String, wavecrate_analysis::aspects::AspectDescriptorSet>,
) -> Result<(), String> {
    let sql = format!(
        "SELECT sample_id, valid_mask, vec FROM similarity_aspect_descriptors
         WHERE model_id = ?1
           AND dim = ?2
           AND dtype = ?3
           AND l2_normed = ?4
           AND sample_id IN ({})",
        placeholder_list(5, sample_ids.len())
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Load aspect descriptors failed: {err}"))?;
    let mut rows = stmt
        .query(params_from_iter(aspect_descriptor_query_values(sample_ids)))
        .map_err(|err| format!("Load aspect descriptors failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Load aspect descriptors failed: {err}"))?
    {
        let sample_id = row
            .get::<_, String>(0)
            .map_err(|err| format!("Load aspect descriptors failed: {err}"))?;
        let valid_mask = row
            .get::<_, i64>(1)
            .map_err(|err| format!("Load aspect descriptors failed: {err}"))?
            as u32;
        let blob = row
            .get::<_, Vec<u8>>(2)
            .map_err(|err| format!("Load aspect descriptors failed: {err}"))?;
        let values = wavecrate_analysis::decode_f32_le_blob(&blob)?;
        let descriptor =
            wavecrate_analysis::aspects::AspectDescriptorSet::from_parts(values, valid_mask)?;
        descriptors.insert(sample_id, descriptor);
    }
    Ok(())
}
