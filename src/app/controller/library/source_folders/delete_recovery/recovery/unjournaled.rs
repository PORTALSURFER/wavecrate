use super::{
    DeleteRecoveryAction, DeleteRecoveryReport, SampleSource, find_unjournaled_staged_roots,
    recovery_entry, restore_staged_folder,
};
use crate::app::controller::library::source_folders::delete_recovery::path_policy;
use std::path::{Path, PathBuf};

pub(super) fn recover_unjournaled_entries(
    source: &SampleSource,
    staging_root: &Path,
    journaled_roots: &[PathBuf],
    report: &mut DeleteRecoveryReport,
) {
    for relative in find_unjournaled_staged_roots(staging_root, &source.root, journaled_roots) {
        let staged = staging_root.join(&relative);
        let original = source.root.join(&relative);
        let outcome = path_policy::validate_relative_path(&relative, "staged_relative")
            .and_then(|_| restore_staged_folder(&staged, &original, staging_root, &source.root));
        report.entries.push(recovery_entry(
            source,
            relative,
            DeleteRecoveryAction::Restore,
            outcome,
        ));
    }
}
