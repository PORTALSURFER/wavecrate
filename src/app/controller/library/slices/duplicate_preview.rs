use super::ops::normalized_selected_indices;
use crate::app::state::WaveformDuplicateCleanupPreview;
use crate::selection::SelectionRange;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DuplicatePreviewMergeResult {
    pub(crate) previews: Vec<WaveformDuplicateCleanupPreview>,
    pub(crate) merged: SelectionRange,
}

pub(crate) fn delete_duplicate_previews(
    previews: &[WaveformDuplicateCleanupPreview],
    selected_indices: &[usize],
) -> (Vec<WaveformDuplicateCleanupPreview>, usize) {
    let mut updated = previews.to_vec();
    let mut removed = 0usize;
    for index in normalized_selected_indices(selected_indices)
        .into_iter()
        .rev()
    {
        if index < updated.len() {
            updated.remove(index);
            removed += 1;
        }
    }
    (updated, removed)
}

pub(crate) fn merge_duplicate_previews(
    previews: &[WaveformDuplicateCleanupPreview],
    selected_indices: &[usize],
) -> Option<DuplicatePreviewMergeResult> {
    let selected = normalized_selected_indices(selected_indices)
        .iter()
        .filter_map(|index| previews.get(*index).copied())
        .collect::<Vec<_>>();
    if selected.len() < 2 {
        return None;
    }
    let merged = SelectionRange::new(
        selected
            .iter()
            .map(|preview| preview.range.start())
            .fold(1.0, f32::min),
        selected
            .iter()
            .map(|preview| preview.range.end())
            .fold(0.0, f32::max),
    );
    if merged.end() <= merged.start() {
        return None;
    }

    let merged_preview = WaveformDuplicateCleanupPreview {
        range: merged,
        group_id: selected
            .iter()
            .map(|preview| preview.group_id)
            .min()
            .unwrap_or(0),
        exempted: selected.iter().all(|preview| preview.exempted),
        represented_window_count: selected
            .iter()
            .map(|preview| preview.represented_window_count)
            .sum(),
    };
    let (mut updated, _) = delete_duplicate_previews(previews, selected_indices);
    updated.push(merged_preview);
    updated.sort_by(|left, right| {
        left.range
            .start()
            .partial_cmp(&right.range.start())
            .unwrap_or(Ordering::Equal)
    });

    Some(DuplicatePreviewMergeResult {
        previews: updated,
        merged,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview(
        start: f32,
        end: f32,
        group_id: usize,
        exempted: bool,
        represented_window_count: usize,
    ) -> WaveformDuplicateCleanupPreview {
        WaveformDuplicateCleanupPreview {
            range: SelectionRange::new(start, end),
            group_id,
            exempted,
            represented_window_count,
        }
    }

    #[test]
    fn delete_duplicate_previews_normalizes_selected_indices() {
        let previews = vec![
            preview(0.1, 0.2, 1, false, 1),
            preview(0.3, 0.4, 2, false, 1),
            preview(0.5, 0.6, 3, false, 1),
        ];

        let (updated, removed) = delete_duplicate_previews(&previews, &[2, 0, 2, 99]);

        assert_eq!(removed, 2);
        assert_eq!(updated, vec![preview(0.3, 0.4, 2, false, 1)]);
    }

    #[test]
    fn merge_duplicate_previews_combines_metadata_and_orders_result() {
        let previews = vec![
            preview(0.5, 0.6, 8, true, 2),
            preview(0.1, 0.2, 3, true, 4),
            preview(0.3, 0.4, 6, true, 1),
        ];

        let result = merge_duplicate_previews(&previews, &[0, 2]).expect("merge");

        assert_eq!(result.merged, SelectionRange::new(0.3, 0.6));
        assert_eq!(result.previews.len(), 2);
        assert_eq!(result.previews[0], preview(0.1, 0.2, 3, true, 4));
        assert_eq!(result.previews[1], preview(0.3, 0.6, 6, true, 3));
    }
}
