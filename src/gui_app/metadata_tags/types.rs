use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct MetadataTagCommit {
    pub(in crate::gui_app) tags: Vec<String>,
    pub(in crate::gui_app) remainder: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct MetadataTagPersistResult {
    pub(in crate::gui_app) tags: Vec<String>,
    pub(in crate::gui_app) assigned: bool,
    pub(in crate::gui_app) result: Result<(), String>,
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct MetadataTagPersistRequest {
    pub(in crate::gui_app) absolute_path: PathBuf,
    pub(in crate::gui_app) source_root: PathBuf,
    pub(in crate::gui_app) relative_path: PathBuf,
    pub(in crate::gui_app) tags: Vec<String>,
    pub(in crate::gui_app) assigned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct MetadataTagCategoryGroup {
    pub(in crate::gui_app) id: &'static str,
    pub(in crate::gui_app) label: &'static str,
    pub(in crate::gui_app) tags: Vec<String>,
    pub(in crate::gui_app) collapsed: bool,
    pub(in crate::gui_app) locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct MetadataTagCompletionOption {
    pub(in crate::gui_app) tag: String,
    pub(in crate::gui_app) category: &'static str,
    pub(in crate::gui_app) selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct MetadataTagDisplayCategory {
    pub(in crate::gui_app) tag: String,
    pub(in crate::gui_app) category_id: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_app) enum MetadataTagInputMode {
    #[default]
    Tag,
    Category {
        pending_tag: String,
    },
}
