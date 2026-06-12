use radiant::prelude as ui;
use std::collections::HashMap;

use super::super::SimilarityBrowserState;
use super::super::file_columns::{sort_file_indices_by_column_kind, sort_kind_for_details_sort};
use super::{FolderBrowserState, FolderEntry};

pub(super) fn sort_file_indices(
    state: &FolderBrowserState,
    folder: &FolderEntry,
    indices: &mut [usize],
) {
    sort_file_indices_by_column_kind(
        sort_kind_for_details_sort(&state.sample_list.file_sort),
        folder,
        indices,
    );
    if state.sample_list.file_sort.direction == ui::SortDirection::Descending {
        indices.reverse();
    }
}

pub(super) fn sort_file_indices_by_similarity(
    state: &FolderBrowserState,
    folder: &FolderEntry,
    indices: &mut [usize],
) {
    let Some(similarity) = state.sample_list.similarity.as_ref() else {
        return;
    };
    let base_order = indices
        .iter()
        .enumerate()
        .map(|(order, index)| (*index, order))
        .collect::<HashMap<_, _>>();
    indices.sort_by(|left, right| {
        similarity_file_order(folder, similarity, &base_order, *left, *right)
    });
}

fn similarity_file_order(
    folder: &FolderEntry,
    similarity: &SimilarityBrowserState,
    base_order: &HashMap<usize, usize>,
    left: usize,
    right: usize,
) -> std::cmp::Ordering {
    let left_file = &folder.files[left];
    let right_file = &folder.files[right];
    match (
        left_file.id == similarity.anchor_id(),
        right_file.id == similarity.anchor_id(),
    ) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
    }

    match (
        similarity.raw_score_for(&left_file.id),
        similarity.raw_score_for(&right_file.id),
    ) {
        (Some(left_score), Some(right_score)) => right_score
            .total_cmp(&left_score)
            .then_with(|| base_order_for(left, base_order).cmp(&base_order_for(right, base_order))),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => base_order_for(left, base_order).cmp(&base_order_for(right, base_order)),
    }
}

fn base_order_for(index: usize, base_order: &HashMap<usize, usize>) -> usize {
    base_order.get(&index).copied().unwrap_or(usize::MAX)
}
