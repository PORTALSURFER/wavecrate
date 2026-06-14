mod discovery_merge;
mod file_entry_metadata;
mod metadata;
mod progress;
mod traversal;
mod verification;

pub(super) use discovery_merge::{merge_scan_discovery, upsert_file, upsert_folder};
pub(super) use file_entry_metadata::file_entry;
pub(super) use metadata::file_entry_for_source_path;
pub(in crate::native_app) use progress::scan_source_with_progress;
pub(in crate::native_app) use traversal::refresh_folder_tree_only;
pub(super) use traversal::{
    default_root_path, load_folder_at_path, load_root_folder, placeholder_folder,
};
pub(in crate::native_app) use verification::verify_direct_folder;
