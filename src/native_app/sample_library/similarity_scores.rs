use std::{collections::HashMap, path::PathBuf};

use radiant::prelude as ui;
use wavecrate::sample_sources::{SourceDatabase, SourceDatabaseConnectionRole, SourceId};
use wavecrate_analysis::{decode_f32_le_blob, similarity::SIMILARITY_MODEL_ID};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, emit_gui_action},
    sample_library::file_actions::sample_path_label,
};

const SQLITE_IN_BATCH_SIZE: usize = 900;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SimilarityScoresResult {
    pub(in crate::native_app) anchor_id: String,
    pub(in crate::native_app) result: Result<HashMap<String, f32>, String>,
}

#[derive(Clone, Debug)]
struct SimilarityScoresRequest {
    source_id: SourceId,
    source_root: PathBuf,
    anchor_id: String,
    anchor_relative_path: PathBuf,
    candidates: Vec<SimilarityScoreCandidate>,
}

#[derive(Clone, Debug)]
struct SimilarityScoreCandidate {
    file_id: String,
    relative_path: PathBuf,
}

impl NativeAppState {
    pub(in crate::native_app) fn queue_similarity_score_resolution(
        &mut self,
        anchor_id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(request) = self.prepare_similarity_scores_request(anchor_id.clone()) else {
            self.ui.status.sample = String::from("Similarity data unavailable for this source");
            return;
        };
        self.ui.status.sample = format!(
            "Resolving similarity for {}",
            sample_path_label(anchor_id.as_str())
        );
        context.business().background("gui-similarity-scores").run(
            move |_| resolve_similarity_scores(request),
            GuiMessage::SimilarityScoresResolved,
        );
    }

    pub(in crate::native_app) fn finish_similarity_scores(
        &mut self,
        result: SimilarityScoresResult,
    ) {
        if self.library.folder_browser.similarity_anchor_id() != Some(result.anchor_id.as_str()) {
            return;
        }
        match result.result {
            Ok(scores) => {
                let count = scores.len().saturating_sub(1);
                self.library
                    .folder_browser
                    .set_similarity_scores(result.anchor_id.clone(), scores);
                self.ui.status.sample = format!(
                    "Resolved {count} similar sample{}",
                    if count == 1 { "" } else { "s" }
                );
            }
            Err(error) => {
                self.ui.status.sample = format!("Similarity scores unavailable: {error}");
                emit_gui_action(
                    "browser.similarity_scores.resolve",
                    Some("browser"),
                    Some(result.anchor_id.as_str()),
                    "error",
                    std::time::Instant::now(),
                    Some(&error),
                );
            }
        }
    }

    fn prepare_similarity_scores_request(
        &self,
        anchor_id: String,
    ) -> Option<SimilarityScoresRequest> {
        let anchor_path = PathBuf::from(&anchor_id);
        let (source_root, anchor_relative_path) = self
            .library
            .folder_browser
            .source_relative_file_path(&anchor_path)?;
        let source_id = SourceId::from_string(self.library.folder_browser.selected_source_id());
        let candidates = self
            .library
            .folder_browser
            .selected_source_audio_files()
            .into_iter()
            .filter_map(|file| {
                let file_path = PathBuf::from(&file.id);
                let (candidate_root, relative_path) = self
                    .library
                    .folder_browser
                    .source_relative_file_path(&file_path)?;
                (candidate_root == source_root).then(|| SimilarityScoreCandidate {
                    file_id: file.id.clone(),
                    relative_path,
                })
            })
            .collect::<Vec<_>>();
        Some(SimilarityScoresRequest {
            source_id,
            source_root,
            anchor_id,
            anchor_relative_path,
            candidates,
        })
    }
}

fn resolve_similarity_scores(request: SimilarityScoresRequest) -> SimilarityScoresResult {
    let anchor_id = request.anchor_id.clone();
    SimilarityScoresResult {
        anchor_id,
        result: resolve_similarity_scores_inner(&request),
    }
}

fn resolve_similarity_scores_inner(
    request: &SimilarityScoresRequest,
) -> Result<HashMap<String, f32>, String> {
    let conn = SourceDatabase::open_connection_with_role(
        &request.source_root,
        SourceDatabaseConnectionRole::UiRead,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))?;
    let anchor_sample_id =
        build_sample_id(request.source_id.as_str(), &request.anchor_relative_path);
    let candidate_sample_ids = request
        .candidates
        .iter()
        .map(|candidate| build_sample_id(request.source_id.as_str(), &candidate.relative_path))
        .collect::<Vec<_>>();
    let mut sample_ids = Vec::with_capacity(candidate_sample_ids.len() + 1);
    sample_ids.push(anchor_sample_id.clone());
    sample_ids.extend(candidate_sample_ids.iter().cloned());
    let mut embeddings = load_embeddings(&conn, &sample_ids)?;
    let anchor = embeddings
        .remove(&anchor_sample_id)
        .ok_or_else(|| String::from("anchor embedding is missing"))?;
    let mut scores = HashMap::new();
    scores.insert(request.anchor_id.clone(), 1.0);
    for (candidate, sample_id) in request.candidates.iter().zip(candidate_sample_ids.iter()) {
        let Some(candidate_embedding) = embeddings.get(sample_id) else {
            continue;
        };
        scores.insert(
            candidate.file_id.clone(),
            cosine_similarity(&anchor, candidate_embedding).clamp(-1.0, 1.0),
        );
    }
    Ok(scores)
}

fn load_embeddings(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
) -> Result<HashMap<String, Vec<f32>>, String> {
    let mut embeddings = HashMap::new();
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        load_embedding_batch(conn, batch, &mut embeddings)?;
    }
    Ok(embeddings)
}

fn load_embedding_batch(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    embeddings: &mut HashMap<String, Vec<f32>>,
) -> Result<(), String> {
    if sample_ids.is_empty() {
        return Ok(());
    }
    let placeholders = std::iter::repeat_n("?", sample_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT sample_id, vec FROM embeddings
         WHERE model_id = ? AND sample_id IN ({placeholders})"
    );
    let mut params = Vec::with_capacity(sample_ids.len() + 1);
    params.push(SIMILARITY_MODEL_ID);
    params.extend(sample_ids.iter().map(String::as_str));
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Prepare embedding lookup failed: {err}"))?;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params))
        .map_err(|err| format!("Query embedding failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Read embedding row failed: {err}"))?
    {
        let sample_id: String = row
            .get(0)
            .map_err(|err| format!("Decode embedding sample id failed: {err}"))?;
        let blob: Vec<u8> = row
            .get(1)
            .map_err(|err| format!("Decode embedding blob failed: {err}"))?;
        embeddings.insert(sample_id, decode_f32_le_blob(&blob)?);
    }
    Ok(())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut a_norm = 0.0;
    let mut b_norm = 0.0;
    for (&left, &right) in a.iter().zip(b) {
        dot += left * right;
        a_norm += left * left;
        b_norm += right * right;
    }
    if a_norm <= f32::EPSILON || b_norm <= f32::EPSILON {
        return 0.0;
    }
    dot / (a_norm.sqrt() * b_norm.sqrt())
}

fn build_sample_id(source_id: &str, relative_path: &std::path::Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}
