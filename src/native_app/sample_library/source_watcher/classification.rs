use notify::{Event, EventKind};
use std::path::Path;

pub(super) fn event_triggers_source_refresh(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

pub(super) fn retain_source_refresh_candidates(event: &mut Event) -> bool {
    if !event_triggers_source_refresh(event) {
        return false;
    }
    event
        .paths
        .retain(|path| path_is_source_refresh_candidate(path, event.kind));
    !event.paths.is_empty()
}

pub(super) fn path_is_source_refresh_candidate(path: &Path, kind: EventKind) -> bool {
    if is_wavecrate_metadata_file(path)
        || is_wavecrate_transient_analysis_path(path)
        || wavecrate_library::sample_sources::is_apple_double_sidecar(path)
    {
        return false;
    }
    matches!(kind, EventKind::Remove(_) | EventKind::Any)
        || path_has_supported_audio_extension(path)
        || path.extension().is_none()
        || path.is_dir()
}

fn is_wavecrate_transient_analysis_path(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        is_tempfile_name(&name, "ann_container") || is_tempfile_name(&name, "ann_dump")
    })
}

fn is_tempfile_name(name: &str, prefix: &str) -> bool {
    let Some(suffix) = name.strip_prefix(prefix) else {
        return false;
    };
    suffix.len() == 6 && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
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
