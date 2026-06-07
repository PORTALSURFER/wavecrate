//! Wavecrate context-menu target model for library-browser commands.

use radiant::gui::types::Point;
use std::path::{Path, PathBuf};
use wavecrate::sample_sources::SampleCollection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum BrowserContextTargetKind {
    Source,
    Folder,
    Sample,
    MetadataTag,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct BrowserContextMenu {
    pub(in crate::native_app) kind: BrowserContextTargetKind,
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) source_id: Option<String>,
    pub(in crate::native_app) source_removable: bool,
    pub(in crate::native_app) metadata_tag: Option<String>,
    pub(in crate::native_app) collection: Option<SampleCollection>,
    pub(in crate::native_app) anchor: Point,
    pub(in crate::native_app) title: String,
}

pub(in crate::native_app) fn target_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(in crate::native_app) fn pane(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "sources",
        BrowserContextTargetKind::Folder => "folder_browser",
        BrowserContextTargetKind::Sample => "browser",
        BrowserContextTargetKind::MetadataTag => "tag_editor",
    }
}

pub(in crate::native_app) fn target_available(
    kind: &BrowserContextTargetKind,
    path: &Path,
) -> bool {
    match kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => path.is_dir(),
        BrowserContextTargetKind::Sample => path.is_file(),
        BrowserContextTargetKind::MetadataTag => true,
    }
}

pub(in crate::native_app) fn missing_target_message(
    kind: &BrowserContextTargetKind,
) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "Source folder is missing",
        BrowserContextTargetKind::Folder => "Folder is missing",
        BrowserContextTargetKind::Sample => "Sample file is missing",
        BrowserContextTargetKind::MetadataTag => "Tag is unavailable",
    }
}
