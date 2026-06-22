use std::path::PathBuf;

use super::{NativeAudioDocumentOpenRejection, NativeAudioDocumentOpenValidation};
use crate::native_app::app::sample_path_label;

pub(super) fn validate_audio_document_open(path: PathBuf) -> NativeAudioDocumentOpenValidation {
    let result = if !path.is_file() {
        Err(NativeAudioDocumentOpenRejection::NotFile {
            message: format!(
                "Open audio failed: {} is not a file",
                sample_path_label(&path)
            ),
        })
    } else if !wavecrate_library::sample_sources::is_supported_audio(&path) {
        Err(NativeAudioDocumentOpenRejection::Unsupported {
            message: format!("Unsupported audio document: {}", sample_path_label(&path)),
        })
    } else {
        Ok(())
    };
    NativeAudioDocumentOpenValidation { path, result }
}
