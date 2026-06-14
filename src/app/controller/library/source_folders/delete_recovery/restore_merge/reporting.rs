//! Report recording for retained-restore merge outcomes.

use super::util::{read_dir_paths, source_relative};
use super::{
    ExistingFileRelocation, RestoredFileDisposition, RestoredFileRecord, RetainedRestoreMergeReport,
};
use std::path::Path;

pub(super) fn record_restored_file(
    report: &mut RetainedRestoreMergeReport,
    original_relative: &Path,
    final_relative: &Path,
    disposition: RestoredFileDisposition,
) {
    report.restored_files.push(RestoredFileRecord {
        original_relative: original_relative.to_path_buf(),
        final_relative: final_relative.to_path_buf(),
        disposition,
    });
}

pub(super) fn record_existing_relocation(
    report: &mut RetainedRestoreMergeReport,
    original_relative: &Path,
    relocated_relative: &Path,
) {
    report.existing_relocations.push(ExistingFileRelocation {
        original_relative: original_relative.to_path_buf(),
        relocated_relative: relocated_relative.to_path_buf(),
    });
}

pub(super) fn record_directory_files(
    report: &mut RetainedRestoreMergeReport,
    final_dir: &Path,
    original_base_relative: &Path,
    disposition: RestoredFileDisposition,
    source_root: &Path,
) -> Result<(), String> {
    for final_child in read_dir_paths(final_dir)? {
        let name = final_child
            .file_name()
            .ok_or_else(|| format!("Restored path missing file name: {}", final_child.display()))?;
        let original_child = original_base_relative.join(name);
        let final_relative = source_relative(source_root, &final_child)?;
        if final_child.is_dir() {
            record_directory_files(
                report,
                &final_child,
                &original_child,
                disposition,
                source_root,
            )?;
        } else if final_child.is_file() {
            record_restored_file(report, &original_child, &final_relative, disposition);
        }
    }
    Ok(())
}
