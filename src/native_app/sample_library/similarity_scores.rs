use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use radiant::prelude as ui;
use wavecrate::sample_sources::{SourceDatabase, SourceDatabaseConnectionRole, SourceId};
use wavecrate_analysis::{
    aspects::{
        ASPECT_DESCRIPTOR_DIM, ASPECT_DESCRIPTOR_DTYPE_F32, ASPECT_DESCRIPTOR_MODEL_ID,
        AspectDescriptorSet, SimilarityAspect,
    },
    decode_f32_le_blob,
    similarity::SIMILARITY_MODEL_ID,
};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, emit_gui_action},
    sample_library::file_actions::sample_path_label,
    sample_library::folder_browser::model::{
        EMPTY_SIMILARITY_ASPECT_STRENGTHS, SimilarityAspectStrengths,
    },
};

const SQLITE_IN_BATCH_SIZE: usize = 900;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SimilarityScoresResult {
    pub(in crate::native_app) anchor_id: String,
    pub(in crate::native_app) result: Result<SimilarityScoresPayload, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SimilarityScoresPayload {
    pub(in crate::native_app) scores_by_file: HashMap<String, f32>,
    pub(in crate::native_app) aspect_scores_by_file: HashMap<String, SimilarityAspectStrengths>,
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
                let scores = self
                    .filter_similarity_scores_to_active_scope(result.anchor_id.as_str(), scores);
                let count = scores.scores_by_file.len().saturating_sub(1);
                self.library
                    .folder_browser
                    .set_similarity_scores_with_aspects(
                        result.anchor_id.clone(),
                        scores.scores_by_file,
                        scores.aspect_scores_by_file,
                    );
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
        let candidates = self.active_similarity_score_candidates(&source_root);
        Some(SimilarityScoresRequest {
            source_id,
            source_root,
            anchor_id,
            anchor_relative_path,
            candidates,
        })
    }

    fn active_similarity_score_candidates(
        &self,
        source_root: &Path,
    ) -> Vec<SimilarityScoreCandidate> {
        self.library
            .folder_browser
            .selected_audio_files_matching_tags(&self.metadata.tags_by_file)
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
            .collect()
    }

    fn filter_similarity_scores_to_active_scope(
        &self,
        anchor_id: &str,
        mut scores: SimilarityScoresPayload,
    ) -> SimilarityScoresPayload {
        let anchor_path = PathBuf::from(anchor_id);
        let Some((source_root, _)) = self
            .library
            .folder_browser
            .source_relative_file_path(&anchor_path)
        else {
            return scores;
        };
        let active_ids = self
            .active_similarity_score_candidates(&source_root)
            .into_iter()
            .map(|candidate| candidate.file_id)
            .collect::<HashSet<_>>();
        if !active_ids.contains(anchor_id) {
            scores
                .scores_by_file
                .retain(|file_id, _| file_id == anchor_id);
            scores
                .aspect_scores_by_file
                .retain(|file_id, _| file_id == anchor_id);
            return scores;
        }
        scores
            .scores_by_file
            .retain(|file_id, _| file_id == anchor_id || active_ids.contains(file_id));
        scores
            .aspect_scores_by_file
            .retain(|file_id, _| file_id == anchor_id || active_ids.contains(file_id));
        scores
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
) -> Result<SimilarityScoresPayload, String> {
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
    let mut aspect_descriptors = load_aspect_descriptors(&conn, &sample_ids)?;
    let anchor = embeddings
        .remove(&anchor_sample_id)
        .ok_or_else(|| String::from("anchor embedding is missing"))?;
    let anchor_aspects = aspect_descriptors.remove(&anchor_sample_id);
    let mut scores = HashMap::new();
    let mut aspect_scores = HashMap::new();
    scores.insert(request.anchor_id.clone(), 1.0);
    if let Some(anchor_aspects) = anchor_aspects.as_ref() {
        aspect_scores.insert(
            request.anchor_id.clone(),
            similarity_aspect_score_row(Some(anchor_aspects), Some(anchor_aspects)),
        );
    }
    for (candidate, sample_id) in request.candidates.iter().zip(candidate_sample_ids.iter()) {
        let Some(candidate_embedding) = embeddings.get(sample_id) else {
            continue;
        };
        scores.insert(
            candidate.file_id.clone(),
            cosine_similarity(&anchor, candidate_embedding).clamp(-1.0, 1.0),
        );
        aspect_scores.insert(
            candidate.file_id.clone(),
            similarity_aspect_score_row(anchor_aspects.as_ref(), aspect_descriptors.get(sample_id)),
        );
    }
    Ok(SimilarityScoresPayload {
        scores_by_file: scores,
        aspect_scores_by_file: aspect_scores,
    })
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

fn load_aspect_descriptors(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
) -> Result<HashMap<String, AspectDescriptorSet>, String> {
    let mut descriptors = HashMap::new();
    for batch in sample_ids.chunks(SQLITE_IN_BATCH_SIZE) {
        load_aspect_descriptor_batch(conn, batch, &mut descriptors)?;
    }
    Ok(descriptors)
}

fn load_aspect_descriptor_batch(
    conn: &rusqlite::Connection,
    sample_ids: &[String],
    descriptors: &mut HashMap<String, AspectDescriptorSet>,
) -> Result<(), String> {
    if sample_ids.is_empty() {
        return Ok(());
    }
    let placeholders = std::iter::repeat_n("?", sample_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT sample_id, valid_mask, vec FROM similarity_aspect_descriptors
         WHERE model_id = ?
           AND dim = ?
           AND dtype = ?
           AND l2_normed = 1
           AND sample_id IN ({placeholders})"
    );
    let mut params = Vec::<rusqlite::types::Value>::with_capacity(sample_ids.len() + 4);
    params.push(rusqlite::types::Value::from(
        ASPECT_DESCRIPTOR_MODEL_ID.to_string(),
    ));
    params.push(rusqlite::types::Value::from(ASPECT_DESCRIPTOR_DIM as i64));
    params.push(rusqlite::types::Value::from(
        ASPECT_DESCRIPTOR_DTYPE_F32.to_string(),
    ));
    params.extend(sample_ids.iter().cloned().map(rusqlite::types::Value::from));
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| format!("Prepare aspect descriptor lookup failed: {err}"))?;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params))
        .map_err(|err| format!("Query aspect descriptors failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Read aspect descriptor row failed: {err}"))?
    {
        let sample_id: String = row
            .get(0)
            .map_err(|err| format!("Decode aspect descriptor sample id failed: {err}"))?;
        let valid_mask = row
            .get::<_, i64>(1)
            .map_err(|err| format!("Decode aspect descriptor mask failed: {err}"))?
            as u32;
        let blob: Vec<u8> = row
            .get(2)
            .map_err(|err| format!("Decode aspect descriptor blob failed: {err}"))?;
        let values = decode_f32_le_blob(&blob)?;
        descriptors.insert(
            sample_id,
            AspectDescriptorSet::from_parts(values, valid_mask)?,
        );
    }
    Ok(())
}

fn similarity_aspect_score_row(
    query: Option<&AspectDescriptorSet>,
    candidate: Option<&AspectDescriptorSet>,
) -> SimilarityAspectStrengths {
    let (Some(query), Some(candidate)) = (query, candidate) else {
        return EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    };
    let mut row = EMPTY_SIMILARITY_ASPECT_STRENGTHS;
    for aspect in SimilarityAspect::ORDER {
        row[aspect.index()] = query
            .cosine_with(candidate, aspect)
            .map(|score| score.clamp(-1.0, 1.0));
    }
    row
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
