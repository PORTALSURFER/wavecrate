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
    pub(super) kind: FileColumnKind,
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) width: f32,
}

impl FileColumn {
    #[cfg(test)]
    pub(in crate::native_app) fn for_tests(id: &str, label: &str, width: f32) -> Self {
        file_column_with(
            FileColumnKind::from_id(id).unwrap_or(FileColumnKind::Name),
            label,
            width,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FileColumnKind {
    Name,
    Rating,
    Collection,
    Extension,
    Size,
    Modified,
    Kind,
    Path,
    Similarity,
}

impl FileColumnKind {
    pub(super) const DEFAULT_VISIBLE: [Self; 6] = [
        Self::Name,
        Self::Rating,
        Self::Collection,
        Self::Extension,
        Self::Size,
        Self::Modified,
    ];

    pub(super) fn from_id(id: &str) -> Option<Self> {
        match id {
            "name" => Some(Self::Name),
            "rating" => Some(Self::Rating),
            "collection" => Some(Self::Collection),
            "extension" => Some(Self::Extension),
            "size" => Some(Self::Size),
            "modified" => Some(Self::Modified),
            "kind" => Some(Self::Kind),
            "path" => Some(Self::Path),
            "similarity" => Some(Self::Similarity),
            _ => None,
        }
    }

    pub(super) fn id(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Rating => "rating",
            Self::Collection => "collection",
            Self::Extension => "extension",
            Self::Size => "size",
            Self::Modified => "modified",
            Self::Kind => "kind",
            Self::Path => "path",
            Self::Similarity => "similarity",
        }
    }

    fn default_label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Rating => "Rating",
            Self::Collection => "Col",
            Self::Extension => "Ext",
            Self::Size => "Size",
            Self::Modified => "Modified",
            Self::Kind => "Kind",
            Self::Path => "Path",
            Self::Similarity => "Sim",
        }
    }

    fn default_width(self) -> f32 {
        match self {
            Self::Name => 240.0,
            Self::Rating => 68.0,
            Self::Collection => 58.0,
            Self::Extension => 54.0,
            Self::Size => 78.0,
            Self::Modified => 112.0,
            Self::Kind => 78.0,
            Self::Path => 220.0,
            Self::Similarity => 58.0,
        }
    }
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
    FileColumnKind::DEFAULT_VISIBLE
        .into_iter()
        .map(file_column)
        .collect()
}

fn file_column(kind: FileColumnKind) -> FileColumn {
    file_column_with(kind, kind.default_label(), kind.default_width())
}

fn file_column_with(kind: FileColumnKind, label: &str, width: f32) -> FileColumn {
    FileColumn {
        kind,
        id: kind.id().to_owned(),
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
