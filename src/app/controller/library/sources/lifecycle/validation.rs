use super::*;

pub(super) fn source_roots_match(existing: &PathBuf, candidate: &PathBuf) -> bool {
    if existing == candidate {
        return true;
    }
    let Ok(existing) = fs::canonicalize(existing) else {
        return false;
    };
    let Ok(candidate) = fs::canonicalize(candidate) else {
        return false;
    };
    existing == candidate
}

pub(super) fn nested_source_conflict_error(
    existing: &PathBuf,
    candidate: &PathBuf,
) -> Option<String> {
    let Ok(existing) = fs::canonicalize(existing) else {
        return None;
    };
    let Ok(candidate) = fs::canonicalize(candidate) else {
        return None;
    };
    if existing == candidate {
        return None;
    }
    if candidate.starts_with(&existing) {
        Some(format!(
            "Source folders cannot be nested: {} is inside existing source {}. Remove or remap the existing source before adding this folder.",
            candidate.display(),
            existing.display()
        ))
    } else if existing.starts_with(&candidate) {
        Some(format!(
            "Source folders cannot be nested: {} contains existing source {}. Remove or remap the existing source before adding this folder.",
            candidate.display(),
            existing.display()
        ))
    } else {
        None
    }
}
