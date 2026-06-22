use std::{io::ErrorKind, path::Path};

use super::entrypoints::{SampleLoadPathValidation, SampleLoadPathValidationRequest};

pub(in crate::native_app::audio::sample_load_actions) fn validate_sample_load_path(
    request: SampleLoadPathValidationRequest,
) -> SampleLoadPathValidation {
    let existing_file = sample_path_is_existing_file(Path::new(&request.path));
    SampleLoadPathValidation::existing(request, existing_file)
}

fn sample_path_is_existing_file(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata.is_file(),
        Err(err) if err.kind() == ErrorKind::NotFound => false,
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "Could not verify selected sample path before load"
            );
            true
        }
    }
}
