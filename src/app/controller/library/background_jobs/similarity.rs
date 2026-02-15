use super::*;

pub(crate) fn handle_similarity_prepared(
    controller: &mut AppController,
    message: jobs::SimilarityPrepResult,
) {
    controller.handle_similarity_prep_result(message);
}
