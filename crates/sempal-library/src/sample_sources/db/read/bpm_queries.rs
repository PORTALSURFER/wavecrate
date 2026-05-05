use std::collections::HashMap;

use rusqlite::OptionalExtension;

use super::super::util::map_sql_error;
use super::super::{SourceDatabase, SourceDbError};

fn decode_optional_bpm(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<Option<f32>> {
    row.get::<_, Option<f64>>(column)
        .map(|value| value.map(|bpm| bpm as f32))
}

impl SourceDatabase {
    /// Fetch the BPM value stored for a specific sample id, when available.
    pub fn bpm_for_sample_id(&self, sample_id: &str) -> Result<Option<f32>, SourceDbError> {
        let bpm = self
            .connection
            .query_row(
                "SELECT bpm FROM samples WHERE sample_id = ?1",
                rusqlite::params![sample_id],
                |row| decode_optional_bpm(row, 0),
            )
            .optional()
            .map_err(map_sql_error)?;
        Ok(bpm.flatten())
    }

    /// Fetch BPM values for a batch of sample ids.
    ///
    /// The returned map includes only sample ids that exist in `samples`; callers
    /// should treat missing ids as "no BPM row available".
    pub fn bpms_for_sample_ids(
        &self,
        sample_ids: &[String],
    ) -> Result<HashMap<String, Option<f32>>, SourceDbError> {
        if sample_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders = std::iter::repeat_n("?", sample_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT sample_id, bpm
             FROM samples
             WHERE sample_id IN ({placeholders})"
        );
        let mut stmt = self.connection.prepare(&sql).map_err(map_sql_error)?;
        let mut rows = stmt
            .query(rusqlite::params_from_iter(sample_ids.iter()))
            .map_err(map_sql_error)?;
        let mut values = HashMap::with_capacity(sample_ids.len());
        while let Some(row) = rows.next().map_err(map_sql_error)? {
            let sample_id: String = row.get(0).map_err(map_sql_error)?;
            let bpm = decode_optional_bpm(row, 1).map_err(map_sql_error)?;
            values.insert(sample_id, bpm);
        }
        Ok(values)
    }
}
