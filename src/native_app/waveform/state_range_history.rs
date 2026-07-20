use wavecrate::selection::SelectionRange;

const RANGE_MERGE_EPSILON: f64 = 1.0e-6;

pub(super) fn insert_merged_range(ranges: &mut Vec<SelectionRange>, selection: SelectionRange) {
    if selection.width_f64() <= 0.0 {
        return;
    }
    ranges.push(SelectionRange::new_precise(
        selection.start_f64(),
        selection.end_f64(),
    ));
    ranges.sort_by(|a, b| a.start_f64().total_cmp(&b.start_f64()));

    let mut merged = Vec::with_capacity(ranges.len());
    for range in ranges.drain(..) {
        let Some(previous) = merged.last_mut() else {
            merged.push(range);
            continue;
        };
        if range.start_f64() <= previous.end_f64() + RANGE_MERGE_EPSILON {
            *previous = SelectionRange::new_precise(
                previous.start_f64(),
                previous.end_f64().max(range.end_f64()),
            );
        } else {
            merged.push(range);
        }
    }
    *ranges = merged;
}
