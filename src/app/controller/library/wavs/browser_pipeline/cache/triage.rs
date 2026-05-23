use crate::sample_sources::Rating;

use super::BrowserPipelineCache;

impl BrowserPipelineCache {
    pub(super) fn update_triage_partition_membership(
        &mut self,
        index: usize,
        previous_tag: Rating,
        next_tag: Rating,
    ) {
        let previous_bucket = triage_bucket_for_rating(previous_tag);
        let next_bucket = triage_bucket_for_rating(next_tag);
        if previous_bucket == next_bucket {
            return;
        }
        remove_index_from_bucket(triage_bucket_rows_mut(self, previous_bucket), index);
        insert_index_into_bucket(triage_bucket_rows_mut(self, next_bucket), index);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TriageBucket {
    Trash,
    Neutral,
    Keep,
}

fn triage_bucket_for_rating(tag: Rating) -> TriageBucket {
    if tag.is_trash() {
        TriageBucket::Trash
    } else if tag.is_keep() {
        TriageBucket::Keep
    } else {
        TriageBucket::Neutral
    }
}

fn triage_bucket_rows_mut(
    cache: &mut BrowserPipelineCache,
    bucket: TriageBucket,
) -> &mut Vec<usize> {
    match bucket {
        TriageBucket::Trash => &mut cache.trash_rows,
        TriageBucket::Neutral => &mut cache.neutral_rows,
        TriageBucket::Keep => &mut cache.keep_rows,
    }
}

fn remove_index_from_bucket(rows: &mut Vec<usize>, index: usize) {
    if let Ok(position) = rows.binary_search(&index) {
        rows.remove(position);
    }
}

fn insert_index_into_bucket(rows: &mut Vec<usize>, index: usize) {
    match rows.binary_search(&index) {
        Ok(_) => {}
        Err(position) => rows.insert(position, index),
    }
}
