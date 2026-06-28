use super::super::super::super::super::state::MapQueryBounds;
use super::super::UmapPointQuery;
use super::{
    bounds::load_umap_bounds, clusters::load_umap_cluster_centroids, points::load_umap_points,
};
use crate::sample_sources::SourceId;
use rusqlite::Connection;

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
        bounds: MapQueryBounds {
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
        bounds: MapQueryBounds {
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
