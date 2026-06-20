use notify::{Event, EventKind};
use std::path::Path;

pub(super) fn event_triggers_source_refresh(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

pub(super) fn path_is_source_refresh_candidate(path: &Path, kind: EventKind) -> bool {
    if is_wavecrate_metadata_file(path)
        || wavecrate_library::sample_sources::is_apple_double_sidecar(path)
    {
        return false;
    }
    matches!(kind, EventKind::Remove(_) | EventKind::Any)
        || path_has_supported_audio_extension(path)
        || path.extension().is_none()
        || path.is_dir()
}

fn path_has_supported_audio_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "wav" | "wave" | "aif" | "aiff"
            )
        })
}

fn is_wavecrate_metadata_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with(wavecrate::sample_sources::db::DB_FILE_NAME)
        || name.starts_with(wavecrate::sample_sources::db::LEGACY_DB_FILE_NAME)
}
