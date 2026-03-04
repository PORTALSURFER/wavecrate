use super::*;

/// Forward similarity-preparation completion into the controller state machine.
pub(crate) fn handle_similarity_prepared(
    controller: &mut AppController,
    message: jobs::SimilarityPrepResult,
) {
    controller.handle_similarity_prep_result(message);
}
