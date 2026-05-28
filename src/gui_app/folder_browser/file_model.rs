use wavecrate::sample_sources::{Rating, SampleCollection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FileEntry {
    pub(in crate::gui_app) id: String,
    pub(in crate::gui_app) name: String,
    pub(in crate::gui_app) stem: String,
    pub(in crate::gui_app) extension: String,
    pub(in crate::gui_app) kind: String,
    pub(in crate::gui_app) size: String,
    pub(in crate::gui_app) size_bytes: u64,
    pub(in crate::gui_app) modified: String,
    pub(in crate::gui_app) modified_rank: u64,
    pub(in crate::gui_app) rating: Rating,
    pub(in crate::gui_app) rating_locked: bool,
    pub(in crate::gui_app) collection: Option<SampleCollection>,
}

impl FileEntry {
    pub(super) fn is_audio(&self) -> bool {
        self.kind == "Audio"
    }
}

pub(super) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
