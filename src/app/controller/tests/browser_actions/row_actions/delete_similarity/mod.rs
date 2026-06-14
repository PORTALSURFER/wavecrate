mod delete_failures;
mod delete_hotkey;
mod delete_selection;
mod duplicate_cleanup;
mod similarity_delete;

use super::*;

fn configure_test_trash(
    controller: &mut crate::app::controller::AppController,
    temp: &tempfile::TempDir,
) -> PathBuf {
    let trash_root = temp.path().join("trash");
    controller.settings.trash_folder = Some(trash_root.clone());
    controller.ui.trash_folder = Some(trash_root.clone());
    trash_root
}
