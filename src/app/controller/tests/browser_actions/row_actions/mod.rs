use super::super::super::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::analysis::vector::encode_f32_le_blob;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, RetainedDeleteBusyEntry, RetainedDeleteResolutionMode,
};
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::state::audio::{AudioLoadIntent, PendingAudio};
use crate::app::controller::ui::hotkeys;
use crate::app::state::FocusContext;
use crate::sample_sources::Rating;
use rusqlite::params;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod delete_similarity;
mod read_only_similarity;
mod retained_recovery;
mod selection_tagging;
mod wav_only_edits;

fn normalize_embedding(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
}

fn insert_similarity_embedding(
    source: &crate::sample_sources::SampleSource,
    relative_path: &str,
    x: f32,
    y: f32,
) {
    let conn = crate::sample_sources::SourceDatabase::open_connection(&source.root)
        .expect("open source DB");
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), Path::new(relative_path));
    let mut embedding = vec![0.0_f32; crate::analysis::similarity::SIMILARITY_DIM];
    embedding[0] = x;
    embedding[1] = y;
    normalize_embedding(&mut embedding);
    let blob = encode_f32_le_blob(&embedding);
    conn.execute(
        "DELETE FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
        params![sample_id, crate::analysis::similarity::SIMILARITY_MODEL_ID,],
    )
    .expect("clear old embedding");
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
        params![
            sample_id,
            crate::analysis::similarity::SIMILARITY_MODEL_ID,
            crate::analysis::similarity::SIMILARITY_DIM as i64,
            blob,
        ],
    )
    .expect("insert embedding");
    crate::analysis::rebuild_ann_index(&conn).expect("rebuild ann index");
}

fn visible_browser_paths(controller: &mut crate::app::controller::AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}

fn set_fast_similarity_metadata(
    source: &crate::sample_sources::SampleSource,
    relative_path: &str,
    fast_sample_rate: u32,
) -> String {
    let conn = analysis_jobs::open_source_db(&source.root).expect("open source db");
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), Path::new(relative_path));
    let fast_version = crate::analysis::version::analysis_version_for_sample_rate(fast_sample_rate);
    conn.execute(
        "UPDATE samples
         SET content_hash = 'fast-prep-hash',
             analysis_version = ?2
         WHERE sample_id = ?1",
        params![sample_id, fast_version],
    )
    .expect("mark fast metadata");
    sample_id
}

fn count_analysis_jobs(source: &crate::sample_sources::SampleSource, sample_id: &str) -> i64 {
    let conn = analysis_jobs::open_source_db(&source.root).expect("open source db");
    conn.query_row(
        "SELECT COUNT(*)
         FROM analysis_jobs
         WHERE sample_id = ?1 AND job_type = 'wav_metadata_v1'",
        params![sample_id],
        |row| row.get(0),
    )
    .expect("count jobs")
}
