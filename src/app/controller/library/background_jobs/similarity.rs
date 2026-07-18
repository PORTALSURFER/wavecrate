use super::*;

/// Forward focused-similarity highlight completions into controller query state.
pub(crate) fn handle_focused_similarity_loaded(
    controller: &mut AppController,
    message: jobs::FocusedSimilarityResult,
) {
    controller.handle_focused_similarity_loaded(message);
}

/// Forward follow-loaded similarity query completions into controller query state.
pub(crate) fn handle_loaded_similarity_query_built(
    controller: &mut AppController,
    message: jobs::LoadedSimilarityQueryResult,
) {
    controller.handle_loaded_similarity_query_built(message);
}
