use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MetadataTagCommit {
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) remainder: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MetadataTagPersistResult {
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) assigned: bool,
    pub(in crate::native_app) result: Result<(), String>,
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct MetadataTagPersistRequest {
    pub(in crate::native_app) absolute_path: PathBuf,
    pub(in crate::native_app) source_root: PathBuf,
    pub(in crate::native_app) relative_path: PathBuf,
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) assigned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MetadataTagCategoryGroup {
    pub(in crate::native_app) id: &'static str,
    pub(in crate::native_app) label: &'static str,
    pub(in crate::native_app) tags: Vec<String>,
    pub(in crate::native_app) collapsed: bool,
    pub(in crate::native_app) locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MetadataTagCompletionOption {
    pub(in crate::native_app) tag: String,
    pub(in crate::native_app) category: &'static str,
    pub(in crate::native_app) selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct MetadataTagDisplayCategory {
    pub(in crate::native_app) tag: String,
    pub(in crate::native_app) category_id: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) enum MetadataTagInputMode {
    #[default]
    Tag,
    Category {
        pending_tag: String,
    },
}
