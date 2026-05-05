use super::*;

/// Forward similarity-preparation completion into the controller state machine.
pub(crate) fn handle_similarity_prepared(
    controller: &mut AppController,
    message: jobs::SimilarityPrepResult,
) {
    controller.handle_similarity_prep_result(message);
}

/// Forward focused-similarity highlight completions into the controller state machine.
pub(crate) fn handle_focused_similarity_loaded(
    controller: &mut AppController,
    message: jobs::FocusedSimilarityResult,
) {
    controller.handle_focused_similarity_loaded(message);
}

/// Forward follow-loaded similarity query completions into the controller state machine.
pub(crate) fn handle_loaded_similarity_query_built(
    controller: &mut AppController,
    message: jobs::LoadedSimilarityQueryResult,
) {
    controller.handle_loaded_similarity_query_built(message);
}
