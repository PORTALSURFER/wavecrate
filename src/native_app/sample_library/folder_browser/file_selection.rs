use std::path::PathBuf;
use wavecrate::sample_sources::Rating;

mod candidate_queries;
mod cross_source_focus;
mod navigation;
mod selection_mutation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct ToggleSelectedSampleResult {
    pub(in crate::native_app) toggled_id: String,
    pub(in crate::native_app) toggled_selected: bool,
    pub(in crate::native_app) focused_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SelectedFileRatingCandidate {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) rating: Rating,
    pub(in crate::native_app) locked: bool,
}
