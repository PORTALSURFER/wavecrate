use std::sync::Arc;

/// Visible-row projection and reverse-lookup caches for the sample browser.
#[derive(Clone, Debug)]
pub struct BrowserViewportState {
    /// Visible rows after applying the active filter.
    pub visible: VisibleRows,
    /// Monotonic revision bumped whenever the visible-row projection changes.
    pub visible_rows_revision: u64,
    /// Revision for the current visible-row reverse lookup map.
    pub visible_row_lookup_revision: u64,
    /// Revision for the current triage-column reverse lookup map.
    pub triage_index_lookup_revision: u64,
    /// First visible-row index currently projected into the native browser window.
    pub render_window_start: usize,
    /// Requested top visible-row index for manual browser viewport scrolling.
    pub view_window_start: usize,
    /// Cached visible-row lookup by absolute wav-entry index.
    pub visible_row_by_absolute: Vec<Option<usize>>,
    /// Visible-row lookup generation per absolute wav-entry index.
    pub visible_row_by_absolute_generation: Vec<u64>,
    /// Cached triage-column lookup by absolute wav-entry index.
    pub triage_index_by_absolute: Vec<Option<SampleBrowserIndex>>,
    /// Triage-column lookup generation per absolute wav-entry index.
    pub triage_index_by_absolute_generation: Vec<u64>,
}

impl Default for BrowserViewportState {
    fn default() -> Self {
        Self {
            visible: VisibleRows::List(Vec::new().into()),
            visible_rows_revision: 0,
            visible_row_lookup_revision: 0,
            triage_index_lookup_revision: 0,
            render_window_start: 0,
            view_window_start: 0,
            visible_row_by_absolute: Vec::new(),
            visible_row_by_absolute_generation: Vec::new(),
            triage_index_by_absolute: Vec::new(),
            triage_index_by_absolute_generation: Vec::new(),
        }
    }
}

/// Visible list representation for the sample browser.
#[derive(Clone, Debug)]
pub enum VisibleRows {
    /// All rows are visible; total stores the count.
    All {
        /// Total number of rows.
        total: usize,
    },
    /// Only the provided indices are visible.
    List(Arc<[usize]>),
}

impl VisibleRows {
    /// Return the number of visible rows.
    pub fn len(&self) -> usize {
        match self {
            VisibleRows::All { total } => *total,
            VisibleRows::List(rows) => rows.len(),
        }
    }

    /// Copy a contiguous visible-window slice into `out`.
    pub fn copy_window_into(&self, start: usize, len: usize, out: &mut Vec<usize>) {
        out.clear();
        if len == 0 {
            return;
        }
        match self {
            VisibleRows::All { total } => {
                if start >= *total {
                    return;
                }
                let end = start.saturating_add(len).min(*total);
                let count = end.saturating_sub(start);
                out.reserve(count);
                out.extend(start..end);
            }
            VisibleRows::List(rows) => {
                let end = start.saturating_add(len).min(rows.len());
                if start >= rows.len() {
                    return;
                }
                out.extend_from_slice(&rows[start..end]);
            }
        }
    }

    /// Map a visible row index to an absolute index.
    pub fn get(&self, row: usize) -> Option<usize> {
        match self {
            VisibleRows::All { total } => (row < *total).then_some(row),
            VisibleRows::List(rows) => rows.get(row).copied(),
        }
    }

    /// Reset the visible rows to an empty list.
    pub fn clear_to_list(&mut self) {
        *self = VisibleRows::List(Vec::new().into());
    }

    /// Iterate over visible absolute indices.
    pub fn iter(&self) -> Box<dyn Iterator<Item = usize> + '_> {
        match self {
            VisibleRows::All { total } => Box::new(0..*total),
            VisibleRows::List(rows) => Box::new(rows.iter().copied()),
        }
    }

    /// Find the visible position for an absolute index.
    pub fn position(&self, index: usize) -> Option<usize> {
        match self {
            VisibleRows::All { total } => (index < *total).then_some(index),
            VisibleRows::List(rows) => rows.iter().position(|i| *i == index),
        }
    }
}

/// Identifies a row inside one of the triage flag columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleBrowserIndex {
    /// Column containing the row.
    pub column: TriageFlagColumn,
    /// Row index within the column.
    pub row: usize,
}

/// Wav triage flag columns: trash, neutral, keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageFlagColumn {
    /// Trash column.
    Trash,
    /// Neutral column.
    Neutral,
    /// Keep column.
    Keep,
}

#[cfg(test)]
mod tests {
    use super::VisibleRows;

    #[test]
    fn visible_rows_all_copy_window_clamps_start_and_len() {
        let rows = VisibleRows::All { total: 7 };
        let mut out = Vec::new();
        rows.copy_window_into(4, 5, &mut out);
        assert_eq!(out, vec![4, 5, 6]);
    }

    #[test]
    fn visible_rows_list_copy_window_is_sliced() {
        let rows = VisibleRows::List(vec![10, 20, 30, 40, 50].into());
        let mut out = Vec::new();
        rows.copy_window_into(1, 3, &mut out);
        assert_eq!(out, vec![20, 30, 40]);
    }

    #[test]
    fn visible_rows_list_copy_window_respects_limits() {
        let rows = VisibleRows::List(vec![10, 20].into());
        let mut out = Vec::new();
        rows.copy_window_into(3, 2, &mut out);
        assert!(out.is_empty());
    }
}
