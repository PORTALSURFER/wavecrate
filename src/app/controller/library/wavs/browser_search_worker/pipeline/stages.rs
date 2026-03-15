//! Staged helpers for search cache refresh, scoring, and visible-row construction.

mod scoring;
mod source_cache;
mod visible_rows;

pub(super) use self::scoring::{
    resolve_query_scores_for_job,
};
pub(super) use self::source_cache::{
    ensure_search_cache_ready_for_job, ensure_search_entries_loaded_for_job,
};
pub(super) use self::visible_rows::{
    BuildVisibleRowsParams, build_fast_path_result_if_applicable, build_visible_rows_for_job,
};
#[cfg(test)]
pub(super) use self::scoring::{
    reusable_prefix_query_scores, try_reuse_cached_query_scores,
};
#[cfg(test)]
pub(super) use self::visible_rows::sort_visible_indices;
