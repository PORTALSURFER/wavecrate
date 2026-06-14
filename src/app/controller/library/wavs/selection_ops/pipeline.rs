use super::super::*;
use super::side_effects::SelectionSideEffects;

struct WavSelectionTarget {
    index: usize,
    path: PathBuf,
    path_changed: bool,
    apply_commit_focus_effects: bool,
    active_loop_enabled: bool,
    entry_looped: bool,
    loop_lock_enabled: bool,
    missing: bool,
}

/// Shared selection implementation with pre-resolved absolute index and path.
pub(super) fn select_wav_known_index_with_options(
    controller: &mut AppController,
    index: usize,
    path: PathBuf,
    rebuild: bool,
    side_effects: SelectionSideEffects,
) {
    if side_effects.commit_pending_age_update {
        controller.defer_pending_age_update_commit_if_path_changes(&path);
    }
    repair_selected_source_context(controller);

    let target = WavSelectionTarget::new(controller, index, path, side_effects);
    prepare_waveform_for_target(controller, &target);
    apply_focus_state(controller, &target);

    if target.apply_commit_focus_effects && !side_effects.refresh_similarity_highlight {
        controller.clear_focused_similarity_highlight();
    }
    if handle_missing_target(controller, &target, rebuild) {
        return;
    }

    publish_commit_or_autoplay(controller, &target, side_effects);
    publish_preview_focus(controller, &target, side_effects);
    if rebuild {
        controller.rebuild_browser_lists();
    }
}

impl WavSelectionTarget {
    fn new(
        controller: &AppController,
        index: usize,
        path: PathBuf,
        side_effects: SelectionSideEffects,
    ) -> Self {
        let path_changed =
            controller.sample_view.wav.selected_wav.as_deref() != Some(path.as_path());
        let commit_focus_pending = controller.ui.browser.selection.commit_focus_pending
            && side_effects.record_focus_history
            && controller.sample_view.wav.selected_wav.as_deref() == Some(path.as_path());
        Self {
            index,
            missing: controller
                .wav_entries
                .entry(index)
                .map(|entry| entry.missing)
                .unwrap_or(false),
            entry_looped: controller
                .wav_entries
                .entry(index)
                .map(|entry| entry.looped)
                .unwrap_or(false),
            active_loop_enabled: controller.ui.waveform.loop_enabled,
            loop_lock_enabled: controller.ui.waveform.loop_lock_enabled,
            apply_commit_focus_effects: path_changed || commit_focus_pending,
            path_changed,
            path,
        }
    }
}

fn repair_selected_source_context(controller: &mut AppController) {
    if controller.current_source().is_some() {
        return;
    }
    if let Some(source_id) = controller
        .selection_state
        .ctx
        .last_selected_browsable_source
        .clone()
        .filter(|id| {
            controller
                .library
                .sources
                .iter()
                .any(|source| &source.id == id)
        })
    {
        controller.selection_state.ctx.selected_source = Some(source_id);
        controller.refresh_sources_ui();
    } else if let Some(first) = controller.library.sources.first().cloned() {
        controller
            .selection_state
            .ctx
            .last_selected_browsable_source = Some(first.id.clone());
        controller.selection_state.ctx.selected_source = Some(first.id);
        controller.refresh_sources_ui();
    }
}

fn prepare_waveform_for_target(controller: &mut AppController, target: &WavSelectionTarget) {
    if target.path_changed {
        let _ = controller.commit_edit_selection_fades();
        controller.ui.waveform.last_start_marker = None;
        if !(target.loop_lock_enabled
            || (controller.settings.feature_flags.autoplay_selection && target.active_loop_enabled))
        {
            controller.ui.waveform.loop_enabled = target.entry_looped;
        }
    }
}

fn apply_focus_state(controller: &mut AppController, target: &WavSelectionTarget) {
    controller.sample_view.wav.selected_wav = Some(target.path.clone());
    controller.ui.browser.selection.last_focused_index = Some(target.index);
    controller.ui.browser.selection.last_focused_path = Some(target.path.clone());
}

fn handle_missing_target(
    controller: &mut AppController,
    target: &WavSelectionTarget,
    rebuild: bool,
) -> bool {
    if !target.missing {
        return false;
    }
    if let Some(source) = controller.current_source() {
        let _ = controller.prune_missing_sample(&source, &target.path);
    }
    controller.show_missing_waveform_notice(&target.path);
    controller.set_status(
        format!("File missing: {}", target.path.display()),
        StatusTone::Warning,
    );
    controller.selection_state.suppress_autoplay_once = false;
    controller.clear_focused_similarity_highlight();
    if rebuild {
        controller.rebuild_browser_lists();
    }
    true
}

fn publish_commit_or_autoplay(
    controller: &mut AppController,
    target: &WavSelectionTarget,
    side_effects: SelectionSideEffects,
) {
    if !side_effects.queue_audio_load {
        controller.selection_state.suppress_autoplay_once = false;
        publish_commit_focus_without_load(controller, target, side_effects);
        return;
    }

    let Some(source) = controller.current_source() else {
        return;
    };
    let autoplay = controller.settings.feature_flags.autoplay_selection
        && !controller.selection_state.suppress_autoplay_once;
    controller.selection_state.suppress_autoplay_once = false;
    let pending_playback = autoplay.then(|| PendingPlayback {
        source_id: source.id.clone(),
        relative_path: target.path.clone(),
        looped: selection_looped(controller, target),
        start_override: None,
        force_loaded_audio: false,
    });
    controller
        .runtime
        .jobs
        .set_pending_playback(pending_playback.clone());
    controller.publish_browser_commit_selection(
        source.id.clone(),
        target.path.clone(),
        target.index,
        target.apply_commit_focus_effects && side_effects.record_focus_history,
        target.apply_commit_focus_effects && side_effects.refresh_similarity_highlight,
        true,
        pending_playback,
    );
}

fn publish_commit_focus_without_load(
    controller: &mut AppController,
    target: &WavSelectionTarget,
    side_effects: SelectionSideEffects,
) {
    if !(target.apply_commit_focus_effects
        && (side_effects.record_focus_history || side_effects.refresh_similarity_highlight))
    {
        return;
    }
    let Some(source_id) = controller.selection_state.ctx.selected_source.clone() else {
        return;
    };
    controller.publish_browser_commit_selection(
        source_id,
        target.path.clone(),
        target.index,
        side_effects.record_focus_history,
        side_effects.refresh_similarity_highlight,
        false,
        None,
    );
}

fn publish_preview_focus(
    controller: &mut AppController,
    target: &WavSelectionTarget,
    side_effects: SelectionSideEffects,
) {
    if side_effects.queue_audio_load {
        return;
    }
    if let Some(source_id) = controller.selection_state.ctx.selected_source.clone() {
        controller.publish_browser_preview_selection(source_id, target.path.clone(), target.index);
    } else {
        controller.ui.browser.selection.commit_focus_pending = true;
    }
}

fn selection_looped(controller: &AppController, target: &WavSelectionTarget) -> bool {
    if target.active_loop_enabled {
        true
    } else if target.path_changed && !target.loop_lock_enabled {
        target.entry_looped
    } else {
        controller.ui.waveform.loop_enabled
    }
}
