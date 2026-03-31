use super::*;
use std::path::Path;

pub(super) fn queue_or_load_pending_playback(
    controller: &mut AppController,
    looped: bool,
    start_override: Option<f64>,
) -> Result<(), String> {
    if let Some(pending) = controller.runtime.jobs.pending_audio() {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: pending.source_id,
                relative_path: pending.relative_path,
                looped,
                start_override,
                force_loaded_audio: false,
            }));
        controller.set_status("Loading audio…", StatusTone::Busy);
        return Ok(());
    }

    let Some(selected) = controller.sample_view.wav.selected_wav.clone() else {
        return Err("Load a .wav file first".into());
    };
    let Some(source) = controller.current_source() else {
        return Err("Load a .wav file first".into());
    };
    let pending_playback = PendingPlayback {
        source_id: source.id.clone(),
        relative_path: selected.clone(),
        looped,
        start_override,
        force_loaded_audio: false,
    };
    controller
        .runtime
        .jobs
        .set_pending_playback(Some(pending_playback.clone()));
    controller.queue_audio_load_for(
        &source,
        &selected,
        AudioLoadIntent::Selection,
        Some(pending_playback),
    )?;
    controller.set_status(format!("Loading {}", selected.display()), StatusTone::Busy);
    Ok(())
}

pub(super) fn queue_or_load_explicit_pending_playback(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
    looped: bool,
    start_override: Option<f64>,
    force_loaded_audio: bool,
) -> Result<(), String> {
    if controller
        .runtime
        .jobs
        .pending_audio()
        .as_ref()
        .is_some_and(|pending| {
            pending.source_id == source.id && pending.relative_path.as_path() == relative_path
        })
    {
        controller
            .runtime
            .jobs
            .set_pending_playback(Some(PendingPlayback {
                source_id: source.id.clone(),
                relative_path: relative_path.to_path_buf(),
                looped,
                start_override,
                force_loaded_audio,
            }));
        controller.set_status("Loading audio…", StatusTone::Busy);
        return Ok(());
    }
    let pending_playback = PendingPlayback {
        source_id: source.id.clone(),
        relative_path: relative_path.to_path_buf(),
        looped,
        start_override,
        force_loaded_audio,
    };
    controller
        .runtime
        .jobs
        .set_pending_playback(Some(pending_playback.clone()));
    controller.queue_audio_load_for(
        source,
        relative_path,
        AudioLoadIntent::Selection,
        Some(pending_playback),
    )?;
    controller.set_status(
        format!("Loading {}", relative_path.display()),
        StatusTone::Busy,
    );
    Ok(())
}

/// Return the currently selected browser sample when browser focus should start new playback.
///
/// This suppresses redundant reloads when the selected browser row already
/// matches the loaded sample for the active source.
pub(super) fn browser_selection_playback_target(
    controller: &mut AppController,
) -> Option<(SampleSource, PathBuf)> {
    if !matches!(
        controller.ui.focus.context,
        crate::app::state::FocusContext::SampleBrowser
    ) {
        return None;
    }
    let selected = controller.sample_view.wav.selected_wav.clone()?;
    let source = controller.current_source()?;
    let loaded_matches = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .is_some_and(|audio| audio.source_id == source.id && audio.relative_path == selected);
    (!loaded_matches).then_some((source, selected))
}
