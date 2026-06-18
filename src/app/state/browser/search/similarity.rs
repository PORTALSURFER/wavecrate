use crate::sample_sources::config::SimilarityAspectSettings;

/// Per-aspect raw similarity scores aligned to one similarity result row.
pub type SimilarityAspectScoreRow = [Option<f32>; wavecrate_analysis::aspects::ASPECT_COUNT];

/// Empty per-aspect row used when descriptors are unavailable.
pub const EMPTY_SIMILARITY_ASPECT_SCORE_ROW: SimilarityAspectScoreRow =
    [None; wavecrate_analysis::aspects::ASPECT_COUNT];

/// Build empty aspect-score rows aligned to a result vector of `len` rows.
pub fn empty_similarity_aspect_score_rows(len: usize) -> Vec<SimilarityAspectScoreRow> {
    vec![EMPTY_SIMILARITY_ASPECT_SCORE_ROW; len]
}

/// Holds the current similar-sounds query context.
#[derive(Clone, Debug)]
pub struct SimilarQuery {
    /// Sample id used as the similarity anchor.
    pub sample_id: String,
    /// Display label for the anchor sample.
    pub label: String,
    /// Entry indices in similarity order.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices`.
    ///
    /// These are blended similarity values from the resolver pipeline. In
    /// practice they are expected to live near `[-1.0, 1.0]`, but callers may
    /// still pass sentinel values outside that range for unavailable matches.
    pub scores: Vec<f32>,
    /// Per-aspect raw similarity scores aligned with `indices`.
    pub aspect_scores: Vec<SimilarityAspectScoreRow>,
    /// Optional anchor index in the visible list.
    pub anchor_index: Option<usize>,
}

impl SimilarQuery {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }

    /// Return the configured effective similarity score for a given entry index.
    pub fn effective_score_for_index(
        &self,
        entry_index: usize,
        controls: &SimilarityAspectSettings,
    ) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.effective_score_at(position, controls)
    }

    /// Return a normalized configured similarity strength for UI display.
    pub fn display_strength_for_index_with_controls(
        &self,
        entry_index: usize,
        controls: &SimilarityAspectSettings,
    ) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        let score = self.effective_score_at(position, controls)?;
        let (min_score, max_score) = self.effective_score_bounds(controls)?;
        let range = max_score - min_score;
        if range <= f32::EPSILON {
            return Some(Self::absolute_display_strength(score));
        }
        Some(((score - min_score) / range).clamp(0.0, 1.0))
    }

    /// Return a normalized similarity strength for UI display.
    ///
    /// The browser bar is intentionally normalized against the current query's
    /// clamped score spread so nearby-but-not-equal results remain visually
    /// distinguishable inside one similarity result set.
    pub fn display_strength_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        let score = self.clamped_score_at(position)?;
        let (min_score, max_score) = self.clamped_score_bounds()?;
        let range = max_score - min_score;
        if range <= f32::EPSILON {
            return Some(Self::absolute_display_strength(score));
        }
        Some(((score - min_score) / range).clamp(0.0, 1.0))
    }

    /// Return the raw score for one aspect and entry index.
    pub fn aspect_score_for_index(
        &self,
        aspect: wavecrate_analysis::aspects::SimilarityAspect,
        entry_index: usize,
    ) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.aspect_scores
            .get(position)
            .and_then(|row| row[aspect.index()])
    }

    /// Return a query-relative display strength for one aspect and entry index.
    pub fn aspect_display_strength_for_index(
        &self,
        aspect: wavecrate_analysis::aspects::SimilarityAspect,
        entry_index: usize,
    ) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        let score = self
            .aspect_scores
            .get(position)
            .and_then(|row| row[aspect.index()])
            .map(|score| score.clamp(-1.0, 1.0))?;
        let (min_score, max_score) = self.clamped_aspect_score_bounds(aspect)?;
        let range = max_score - min_score;
        if range <= f32::EPSILON {
            return Some(Self::absolute_display_strength(score));
        }
        Some(((score - min_score) / range).clamp(0.0, 1.0))
    }

    fn clamped_score_at(&self, position: usize) -> Option<f32> {
        self.scores
            .get(position)
            .copied()
            .map(|score| score.clamp(-1.0, 1.0))
    }

    fn effective_score_at(
        &self,
        position: usize,
        controls: &SimilarityAspectSettings,
    ) -> Option<f32> {
        let raw_score = self.scores.get(position).copied();
        let row = self
            .aspect_scores
            .get(position)
            .unwrap_or(&EMPTY_SIMILARITY_ASPECT_SCORE_ROW);
        controls.effective_score(raw_score, row)
    }

    fn clamped_score_bounds(&self) -> Option<(f32, f32)> {
        let mut scores = self
            .scores
            .iter()
            .copied()
            .map(|score| score.clamp(-1.0, 1.0));
        let first = scores.next()?;
        let mut min_score = first;
        let mut max_score = first;
        for score in scores {
            min_score = min_score.min(score);
            max_score = max_score.max(score);
        }
        Some((min_score, max_score))
    }

    fn effective_score_bounds(&self, controls: &SimilarityAspectSettings) -> Option<(f32, f32)> {
        let mut scores = (0..self.scores.len())
            .filter_map(|position| self.effective_score_at(position, controls));
        let first = scores.next()?;
        let mut min_score = first;
        let mut max_score = first;
        for score in scores {
            min_score = min_score.min(score);
            max_score = max_score.max(score);
        }
        Some((min_score, max_score))
    }

    fn clamped_aspect_score_bounds(
        &self,
        aspect: wavecrate_analysis::aspects::SimilarityAspect,
    ) -> Option<(f32, f32)> {
        let mut scores = self
            .aspect_scores
            .iter()
            .filter_map(|row| row[aspect.index()])
            .map(|score| score.clamp(-1.0, 1.0));
        let first = scores.next()?;
        let mut min_score = first;
        let mut max_score = first;
        for score in scores {
            min_score = min_score.min(score);
            max_score = max_score.max(score);
        }
        Some((min_score, max_score))
    }

    fn absolute_display_strength(score: f32) -> f32 {
        let normalized = ((score.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
        normalized.powf(2.0)
    }
}

/// Highlight metadata for near-duplicate rows relative to the focused sample.
#[derive(Clone, Debug)]
pub struct FocusedSimilarity {
    /// Sample id used as the highlight anchor.
    pub sample_id: String,
    /// Entry indices for near-duplicate matches.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices`.
    pub scores: Vec<f32>,
    /// Per-aspect raw similarity scores aligned with `indices`.
    pub aspect_scores: Vec<SimilarityAspectScoreRow>,
    /// Absolute index of the focused sample, when known.
    pub anchor_index: Option<usize>,
}

impl FocusedSimilarity {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }

    /// Return the raw score for one aspect and entry index.
    pub fn aspect_score_for_index(
        &self,
        aspect: wavecrate_analysis::aspects::SimilarityAspect,
        entry_index: usize,
    ) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.aspect_scores
            .get(position)
            .and_then(|row| row[aspect.index()])
    }
}
