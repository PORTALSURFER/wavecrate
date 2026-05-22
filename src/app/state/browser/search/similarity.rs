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
    /// Optional anchor index in the visible list.
    pub anchor_index: Option<usize>,
}

impl SimilarQuery {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
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

    fn clamped_score_at(&self, position: usize) -> Option<f32> {
        self.scores
            .get(position)
            .copied()
            .map(|score| score.clamp(-1.0, 1.0))
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
    /// Absolute index of the focused sample, when known.
    pub anchor_index: Option<usize>,
}

impl FocusedSimilarity {
    /// Return the raw similarity score for a given entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self.indices.iter().position(|idx| *idx == entry_index)?;
        self.scores.get(position).copied()
    }
}
