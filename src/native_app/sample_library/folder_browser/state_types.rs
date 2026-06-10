use std::path::PathBuf;

use super::FolderEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SourceEntry {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(super) root: PathBuf,
    pub(super) root_folder: Option<FolderEntry>,
    pub(in crate::native_app) loading_task: Option<u64>,
}

impl SourceEntry {
    pub(in crate::native_app) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        root: PathBuf,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            root,
            root_folder: None,
            loading_task: None,
        }
    }

    pub(super) fn is_default_assets_source(&self) -> bool {
        self.id == "assets" && self.root.ends_with("assets")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderRenameEdit {
    pub(super) folder_id: String,
    pub(super) draft: String,
    pub(super) input_id: u64,
    pub(super) kind: FolderRenameKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderRenameKind {
    Rename,
    Create { parent_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileRenameEdit {
    pub(super) file_id: String,
    pub(super) draft: String,
    pub(super) input_id: u64,
    pub(super) selection_start: usize,
    pub(super) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct FileColumn {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) width: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderBrowserDrag {
    Folder { folder_id: String },
    Files { file_ids: Vec<String> },
    ExtractedFile { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct SimilarityBrowserState {
    anchor_id: String,
    scores_by_file: std::collections::HashMap<String, f32>,
    score_bounds: Option<(f32, f32)>,
}

impl SimilarityBrowserState {
    pub(in crate::native_app) fn new(anchor_id: String) -> Self {
        Self::with_scores(anchor_id, std::collections::HashMap::new())
    }

    pub(in crate::native_app) fn with_scores(
        anchor_id: String,
        mut scores_by_file: std::collections::HashMap<String, f32>,
    ) -> Self {
        scores_by_file.retain(|_, score| score.is_finite());
        scores_by_file.insert(anchor_id.clone(), 1.0);
        let score_bounds = score_bounds(scores_by_file.values().copied());
        Self {
            anchor_id,
            scores_by_file,
            score_bounds,
        }
    }

    pub(in crate::native_app) fn anchor_id(&self) -> &str {
        &self.anchor_id
    }

    pub(in crate::native_app) fn raw_score_for(&self, file_id: &str) -> Option<f32> {
        self.scores_by_file.get(file_id).copied()
    }

    pub(in crate::native_app) fn display_strength_for(&self, file_id: &str) -> Option<f32> {
        let score = self.raw_score_for(file_id)?.clamp(-1.0, 1.0);
        let (min_score, max_score) = self.score_bounds?;
        let range = max_score - min_score;
        if range <= f32::EPSILON {
            return Some(absolute_display_strength(score));
        }
        Some(((score - min_score) / range).clamp(0.0, 1.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct VisibleFolder {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) name: String,
    pub(in crate::native_app) depth: usize,
    pub(in crate::native_app) is_source_root: bool,
    pub(in crate::native_app) has_children: bool,
    pub(in crate::native_app) expanded: bool,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) drag_source: bool,
    pub(in crate::native_app) drop_candidate: bool,
    pub(in crate::native_app) drop_target: bool,
    pub(in crate::native_app) drop_target_active: bool,
    pub(in crate::native_app) rename_draft: Option<String>,
    pub(in crate::native_app) rename_input_id: Option<u64>,
}

pub(super) fn default_file_columns() -> Vec<FileColumn> {
    vec![
        file_column("name", "Name", 240.0),
        file_column("rating", "Rating", 68.0),
        file_column("collection", "Col", 58.0),
        file_column("extension", "Ext", 54.0),
        file_column("size", "Size", 78.0),
        file_column("modified", "Modified", 112.0),
    ]
}

fn file_column(id: &str, label: &str, width: f32) -> FileColumn {
    FileColumn {
        id: id.to_owned(),
        label: label.to_owned(),
        width,
    }
}

fn score_bounds(scores: impl IntoIterator<Item = f32>) -> Option<(f32, f32)> {
    let mut scores = scores.into_iter().map(|score| score.clamp(-1.0, 1.0));
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
