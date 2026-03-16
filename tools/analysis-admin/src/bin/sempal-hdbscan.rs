//! Developer utility to build HDBSCAN clusters from embeddings.

use rusqlite::Connection;
use sempal_analysis_admin::cli_support;

#[path = "sempal-hdbscan/args.rs"]
mod args;
#[path = "sempal-hdbscan/clustering.rs"]
mod clustering;
#[path = "sempal-hdbscan/embeddings.rs"]
mod embeddings;
#[path = "sempal-hdbscan/writeback.rs"]
mod writeback;

use self::args::parse_args;

fn main() {
    cli_support::run_command(run);
}

fn run() -> Result<(), String> {
    let Some(options) = parse_args(std::env::args().skip(1).collect())? else {
        return Ok(());
    };
    let db_path = cli_support::resolve_library_db_path(options.db_path.as_deref())?;
    let conn = Connection::open(&db_path).map_err(|err| format!("Open DB failed: {err}"))?;
    let (sample_ids, data) = embeddings::load_embeddings(&conn, &options.model_id)?;

    if data.is_empty() {
        return Err("No data points found for clustering".to_string());
    }

    println!(
        "Loaded {} samples for HDBSCAN (method=embedding)",
        data.len()
    );

    let (labels, stats) = clustering::cluster_embeddings(&data, &options)?;
    println!(
        "Clusters: {} (noise: {} / {:.2}%, size min/max: {}/{})",
        stats.cluster_count,
        stats.noise_count,
        stats.noise_ratio * 100.0,
        stats.min_cluster_size,
        stats.max_cluster_size
    );

    let mut conn = conn;
    let inserted = writeback::write_clusters(
        &mut conn,
        &sample_ids,
        &labels,
        &options.model_id,
        "embedding",
        "",
    )?;
    println!(
        "Wrote {} cluster assignments for method=embedding",
        inserted
    );
    Ok(())
}
