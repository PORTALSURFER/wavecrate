//! Geometry helpers for mapping pointer positions onto stacked row rects.

use crate::gui::types::{Point, Rect};

/// Resolve the first row index whose rect contains the pointer.
#[cfg(test)]
pub(crate) fn compute_row_index_at_point(row_rects: &[Rect], point: Point) -> Option<usize> {
    row_rects.iter().position(|rect| rect.contains(point))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_index_resolves_matching_rect() {
        let rows = vec![
            Rect::from_min_max(Point::new(8.0, 100.0), Point::new(200.0, 120.0)),
            Rect::from_min_max(Point::new(8.0, 124.0), Point::new(200.0, 144.0)),
            Rect::from_min_max(Point::new(8.0, 148.0), Point::new(200.0, 168.0)),
        ];
        assert_eq!(
            compute_row_index_at_point(&rows, Point::new(24.0, 132.0)),
            Some(1)
        );
    }

    #[test]
    fn row_index_returns_none_for_gap_point() {
        let rows = vec![
            Rect::from_min_max(Point::new(8.0, 100.0), Point::new(200.0, 120.0)),
            Rect::from_min_max(Point::new(8.0, 124.0), Point::new(200.0, 144.0)),
        ];
        assert_eq!(
            compute_row_index_at_point(&rows, Point::new(24.0, 122.0)),
            None
        );
    }

    #[test]
    fn row_index_returns_none_for_empty_rows() {
        let rows: Vec<Rect> = Vec::new();
        assert_eq!(
            compute_row_index_at_point(&rows, Point::new(24.0, 122.0)),
            None
        );
    }
}
