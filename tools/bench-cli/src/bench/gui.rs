//! GUI-oriented benchmark scenarios for the native controller.

/// Interaction attribution summaries for GUI projection benchmarks.
mod attribution;
/// Focused interaction latency scenarios used by the GUI benchmark harness.
mod interactions;
/// Rebuild-cause probes for retained projection attribution reporting.
mod rebuild_probe;
/// Result/report types and assembly helpers for GUI benchmark runs.
mod report;
/// Scenario registry and staged latency collection for GUI benchmark runs.
mod scenario_registry;
/// Segment counter probes for retained projection attribution reporting.
mod segment_probe;
/// Seeded workspace helpers that isolate synthetic GUI benchmark sources.
mod workspace;

use self::attribution::{GuiInteractionRebuildCauseAttribution, GuiInteractionSegmentAttribution};
use self::interactions::execute_interaction_step;
use self::rebuild_probe::collect_interaction_rebuild_cause_attribution;
pub(super) use self::report::GuiBenchResult;
use self::report::assemble_gui_bench_result;
use self::scenario_registry::collect_gui_scenario_metrics;
use self::segment_probe::collect_interaction_segment_attribution;
use self::workspace::{build_controller_with_db_rows, seed_rows};
use super::{options::BenchOptions, stats};

/// Run GUI benchmark actions and summarize performance characteristics.
pub(super) fn run(options: &BenchOptions) -> Result<GuiBenchResult, String> {
    let mut scenario_workspace = build_controller_with_db_rows(options)?;
    let seeded_rows = seed_rows(&mut scenario_workspace.controller, options.gui_rows)?;
    let scenario_metrics = collect_gui_scenario_metrics(
        options,
        scenario_workspace.controller,
        execute_interaction_step,
    )?;
    let mut attribution_workspace = build_controller_with_db_rows(options)?;
    let _ = seed_rows(&mut attribution_workspace.controller, options.gui_rows)?;
    let interaction_segment_attribution = Some(collect_interaction_segment_attribution(
        options,
        &mut attribution_workspace.controller,
    )?);
    let interaction_rebuild_cause_attribution =
        Some(collect_interaction_rebuild_cause_attribution(
            options,
            &mut attribution_workspace.controller,
        )?);
    Ok(assemble_gui_bench_result(
        seeded_rows,
        scenario_metrics,
        interaction_segment_attribution,
        interaction_rebuild_cause_attribution,
    ))
}

/// GUI benchmark behavior and interaction sequencing tests.
#[cfg(test)]
#[path = "gui_tests.rs"]
mod tests;
