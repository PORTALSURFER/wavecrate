//! Similarity-map report and validation helpers.

use serde::Serialize;
use std::path::{Path, PathBuf};

use super::LayoutPoint;

/// Report summarizing one similarity-map layout build.
///
/// The current layout implementation uses t-SNE while persisting into the
/// existing `layout_umap` schema for compatibility.
#[derive(Debug, Serialize)]
pub struct MapLayoutReport {
    /// Total number of embeddings considered.
    pub total: usize,
    /// Number of embeddings included in the final layout.
    pub valid: usize,
    /// Number of embeddings skipped due to invalid data.
    pub invalid: usize,
    /// Ratio of valid points to total points.
    pub coverage_ratio: f32,
    /// Minimum X coordinate of the layout.
    pub x_min: f32,
    /// Maximum X coordinate of the layout.
    pub x_max: f32,
    /// Minimum Y coordinate of the layout.
    pub y_min: f32,
    /// Maximum Y coordinate of the layout.
    pub y_max: f32,
}

/// Legacy compatibility alias for the similarity-map layout report type.
pub type UmapReport = MapLayoutReport;

/// Return the default JSON report path for one layout build report.
///
/// The on-disk filename still uses the historical `umap_report_*` prefix so it
/// remains compatible with existing scripts and generated artifacts.
pub fn default_layout_report_path(db_path: &Path, layout_version: &str) -> PathBuf {
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("umap_report_{}.json", layout_version))
}

/// Serialize and write a layout report to disk as pretty-printed JSON.
pub fn write_layout_report(path: &Path, report: &MapLayoutReport) -> Result<(), String> {
    let data = serde_json::to_vec_pretty(report)
        .map_err(|err| format!("Serialize report failed: {err}"))?;
    std::fs::write(path, data).map_err(|err| format!("Write report failed: {err}"))?;
    Ok(())
}

/// Validate one projected layout and summarize its coverage and bounds.
pub(super) fn validate_layout(
    layout: &[LayoutPoint],
    min_coverage: f32,
) -> Result<MapLayoutReport, String> {
    let summary = summarize_layout(layout);
    ensure_coverage(summary.coverage_ratio, min_coverage)?;
    if summary.valid == 0 {
        return Err("Similarity map layout produced no valid coordinates".to_string());
    }
    Ok(summary)
}

fn summarize_layout(layout: &[LayoutPoint]) -> MapLayoutReport {
    let mut summary = empty_summary(layout.len());
    for coords in layout {
        if let Some((x, y)) = finite_coords(coords) {
            summary.valid += 1;
            summary.x_min = summary.x_min.min(x);
            summary.x_max = summary.x_max.max(x);
            summary.y_min = summary.y_min.min(y);
            summary.y_max = summary.y_max.max(y);
        }
    }
    summary.invalid = summary.total.saturating_sub(summary.valid);
    summary.coverage_ratio = coverage_ratio(summary.total, summary.valid);
    summary
}

fn empty_summary(total: usize) -> MapLayoutReport {
    MapLayoutReport {
        total,
        valid: 0,
        invalid: 0,
        coverage_ratio: 0.0,
        x_min: f32::INFINITY,
        x_max: f32::NEG_INFINITY,
        y_min: f32::INFINITY,
        y_max: f32::NEG_INFINITY,
    }
}

fn finite_coords(coords: &LayoutPoint) -> Option<(f32, f32)> {
    let x = coords[0];
    let y = coords[1];
    if x.is_finite() && y.is_finite() {
        return Some((x, y));
    }
    None
}

fn coverage_ratio(total: usize, valid: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    valid as f32 / total as f32
}

fn ensure_coverage(coverage_ratio: f32, min_coverage: f32) -> Result<(), String> {
    if coverage_ratio < min_coverage {
        return Err(format!(
            "Similarity map layout coverage {:.2}% below threshold {:.2}%",
            coverage_ratio * 100.0,
            min_coverage * 100.0
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_report_path_uses_parent_directory_and_version() {
        let path = default_layout_report_path(Path::new("/tmp/library/source.db"), "v2");
        assert_eq!(path, PathBuf::from("/tmp/library/umap_report_v2.json"));
    }

    #[test]
    fn write_report_serializes_pretty_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.json");
        let report = MapLayoutReport {
            total: 4,
            valid: 4,
            invalid: 0,
            coverage_ratio: 1.0,
            x_min: -1.0,
            x_max: 1.0,
            y_min: -2.0,
            y_max: 2.0,
        };

        write_layout_report(&path, &report).expect("write report");
        let written = std::fs::read_to_string(&path).expect("read report");
        assert!(written.contains("\"coverage_ratio\": 1.0"));
        assert!(written.contains('\n'));
    }

    #[test]
    fn validate_layout_rejects_low_coverage() {
        let layout = [[0.0, 1.0], [f32::NAN, 2.0]];
        let err = validate_layout(&layout, 0.75).unwrap_err();
        assert!(err.contains("coverage"));
    }
}
