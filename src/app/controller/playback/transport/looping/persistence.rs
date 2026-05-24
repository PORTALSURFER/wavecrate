use super::*;

/// Browser action rows used for multi-sample loop/BPM metadata writes.
struct LoopActionTargets {
    /// Primary browser row (loaded sample when visible).
    primary_row: Option<usize>,
    /// Action target paths (selection plus primary when needed).
    paths: Vec<std::path::PathBuf>,
}

struct LoadedSampleMetadataTarget {
    source: crate::sample_sources::SampleSource,
    relative_path: std::path::PathBuf,
}

#[derive(Clone, Copy)]
struct PositiveBpm(f32);

impl PositiveBpm {
    fn new(value: Option<f32>) -> Option<Self> {
        let bpm = value?;
        bpm.is_finite()
            .then_some(bpm)
            .filter(|bpm| *bpm > 0.0)
            .map(Self)
    }

    fn value(self) -> f32 {
        self.0
    }
}

/// Persist loop marker state to selected browser rows or loaded-sample fallback.
pub(super) fn persist_loop_toggle_markers(controller: &mut AppController, state: LoopToggleState) {
    let action_targets = loop_action_targets(controller);
    if !action_targets.paths.is_empty() {
        persist_browser_loop_markers(controller, &action_targets, state);
    } else {
        persist_loaded_sample_loop_marker(controller, state.loop_enabled);
        if state.toggled_to_enabled() {
            persist_loaded_sample_bpm(controller);
        }
    }
}

/// Resolve action paths targeted by loop metadata updates.
fn loop_action_targets(controller: &mut AppController) -> LoopActionTargets {
    let loaded_path = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.relative_path.clone());
    let primary_row = loaded_path
        .as_ref()
        .and_then(|path| controller.visible_row_for_path(path));
    let paths = primary_row
        .map(|row| controller.browser_action_paths_from_primary(row))
        .unwrap_or_default();
    LoopActionTargets { primary_row, paths }
}

/// Persist loop markers (and initial BPM when enabling) across targeted browser rows.
fn persist_browser_loop_markers(
    controller: &mut AppController,
    action_targets: &LoopActionTargets,
    state: LoopToggleState,
) {
    let primary_row = action_targets.primary_row.unwrap_or(0);
    if let Err(err) = controller.set_loop_marker_browser_sample_paths(
        &action_targets.paths,
        state.loop_enabled,
        primary_row,
    ) {
        tracing::warn!("Failed to update loop markers for browser samples: {err}");
    }
    if state.toggled_to_enabled()
        && let Some(bpm) = PositiveBpm::new(controller.ui.waveform.bpm_value)
    {
        persist_browser_bpm_markers(controller, action_targets, bpm, primary_row);
    }
}

fn persist_browser_bpm_markers(
    controller: &mut AppController,
    action_targets: &LoopActionTargets,
    bpm: PositiveBpm,
    primary_row: usize,
) {
    let bpm = bpm.value();
    if let Some(source_id) = controller.selected_source_id() {
        let cache = controller
            .ui_cache
            .browser
            .bpm_values
            .entry(source_id)
            .or_default();
        for path in &action_targets.paths {
            cache.insert(path.clone(), Some(bpm));
        }
        controller.mark_browser_row_metadata_projection_revision_dirty();
    }
    if let Err(err) =
        controller.set_bpm_browser_sample_paths(&action_targets.paths, bpm, primary_row)
    {
        tracing::warn!("Failed to save BPM to browser samples: {err}");
    }
}

/// Persist loop marker state for the loaded sample when no browser rows are actionable.
fn persist_loaded_sample_loop_marker(controller: &mut AppController, loop_enabled: bool) {
    let loop_marker_update = loaded_sample_update_target(controller);
    if let Some(target) = loop_marker_update
        && let Err(err) = controller.set_sample_looped_for_source(
            &target.source,
            &target.relative_path,
            loop_enabled,
            false,
        )
    {
        tracing::warn!("Failed to update loop marker: {err}");
    }
}

/// Persist BPM metadata for the loaded sample when loop-enable falls back to sample scope.
fn persist_loaded_sample_bpm(controller: &mut AppController) {
    let Some(bpm) = PositiveBpm::new(controller.ui.waveform.bpm_value) else {
        return;
    };
    let Some(target) = loaded_sample_update_target(controller) else {
        return;
    };
    persist_loaded_sample_bpm_target(controller, target, bpm);
}

fn persist_loaded_sample_bpm_target(
    controller: &mut AppController,
    target: LoadedSampleMetadataTarget,
    bpm: PositiveBpm,
) {
    let bpm = bpm.value();
    let before_bpm = controller
        .ui_cache
        .browser
        .bpm_values
        .get(&target.source.id)
        .and_then(|cache| cache.get(&target.relative_path).copied().flatten());
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(target.source.id.clone())
        .or_default()
        .insert(target.relative_path.clone(), Some(bpm));
    controller.queue_metadata_mutation(
        &target.source,
        Vec::new(),
        vec![
            crate::app::controller::jobs::AnalysisMetadataMutationOp::SetBpm {
                relative_path: target.relative_path.clone(),
                bpm: Some(bpm),
            },
        ],
        vec![
            crate::app::controller::state::runtime::MetadataRollback::Bpm {
                relative_path: target.relative_path,
                before_bpm,
                expected_bpm: Some(bpm),
            },
        ],
        false,
    );
    controller.mark_browser_row_metadata_projection_revision_dirty();
}

fn loaded_sample_update_target(controller: &AppController) -> Option<LoadedSampleMetadataTarget> {
    controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .and_then(|loaded_audio| {
            controller
                .library
                .sources
                .iter()
                .find(|source| source.id == loaded_audio.source_id)
                .map(|source| LoadedSampleMetadataTarget {
                    source: source.clone(),
                    relative_path: loaded_audio.relative_path.clone(),
                })
        })
}
