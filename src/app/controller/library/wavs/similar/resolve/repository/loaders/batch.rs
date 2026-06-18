use rusqlite::types::Value;

pub(crate) const SQLITE_IN_BATCH_SIZE: usize = 900;

pub(super) fn placeholder_list(start_index: usize, count: usize) -> String {
    (0..count)
        .map(|offset| format!("?{}", start_index + offset))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn sample_id_values(sample_ids: &[String]) -> Vec<Value> {
    sample_ids
        .iter()
        .cloned()
        .map(Value::from)
        .collect::<Vec<_>>()
}

pub(super) fn embedding_query_values(sample_ids: &[String]) -> Vec<Value> {
    let mut params = Vec::with_capacity(sample_ids.len() + 1);
    params.push(Value::from(
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID.to_string(),
    ));
    params.extend(sample_ids.iter().cloned().map(Value::from));
    params
}

pub(super) fn aspect_descriptor_query_values(sample_ids: &[String]) -> Vec<Value> {
    let mut params = Vec::with_capacity(sample_ids.len() + 4);
    params.push(Value::from(
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID.to_string(),
    ));
    params.push(Value::from(
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
    ));
    params.push(Value::from(
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32.to_string(),
    ));
    params.push(Value::from(1_i64));
    params.extend(sample_ids.iter().cloned().map(Value::from));
    params
}
