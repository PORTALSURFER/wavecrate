use super::*;

pub(super) fn record_fast_audition_decision(
    path: &str,
    options: FastAuditionOptions,
    probe: Option<FastAuditionProbe>,
    outcome: InstantAuditionOutcome,
    started_at: Option<Instant>,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "fast_audition",
        event = "fast_audition.decision",
        path,
        origin = options.origin,
        probe = probe.map(FastAuditionProbe::as_str).unwrap_or("none"),
        outcome = outcome.as_str(),
        record_history = options.record_history,
        allow_sidecar_lookup = options.allow_sidecar_lookup,
        queue_preview_decode = options.queue_preview_decode,
        prefer_preview_decode = options.prefer_preview_decode,
        allow_file_backed_source = options.allow_file_backed_source,
        replace_policy = ?options.replace_policy,
        elapsed_ms = starmap_telemetry::elapsed_since(started_at)
            .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
            .unwrap_or(0.0),
        "Fast audition decision"
    );
}

fn sample_browser_display_mode_str(mode: SampleBrowserDisplayMode) -> &'static str {
    match mode {
        SampleBrowserDisplayMode::List => "list",
        SampleBrowserDisplayMode::Map => "starmap",
    }
}

pub(super) fn record_preview_audition_warm_plan(
    display_mode: SampleBrowserDisplayMode,
    outcome: &'static str,
    reason: Option<&'static str>,
    plan: Option<&PreviewAuditionWarmPlan>,
    elapsed: Option<Duration>,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    tracing::info!(
        target: "perf::audio_start",
        module = "preview_audition_warm",
        event = "preview_audition.warm_plan",
        display_mode = sample_browser_display_mode_str(display_mode),
        outcome,
        reason = reason.unwrap_or(""),
        scheduled = plan.map(|plan| plan.paths.len()).unwrap_or(0),
        inspected = plan.map(|plan| plan.inspected_count).unwrap_or(0),
        candidates = plan.map(|plan| plan.candidate_count).unwrap_or(0),
        eligible = plan.map(|plan| plan.eligible_count).unwrap_or(0),
        starmap_cells = plan.map(|plan| plan.starmap_cell_count).unwrap_or(0),
        starmap_visited_cells = plan
            .map(|plan| plan.starmap_visited_cell_count)
            .unwrap_or(0),
        starmap_signature = plan
            .and_then(|plan| plan.starmap_signature)
            .unwrap_or_default(),
        list_signature = plan
            .and_then(|plan| plan.list_signature)
            .unwrap_or_default(),
        starmap_remaining_budget = plan
            .and_then(|plan| plan.starmap_remaining_budget)
            .unwrap_or_default(),
        list_remaining_budget = plan
            .and_then(|plan| plan.list_remaining_budget)
            .unwrap_or_default(),
        elapsed_ms = elapsed
            .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
            .unwrap_or(0.0),
        "Preview audition warm plan"
    );
}

pub(super) fn record_preview_audition_warm_finished(
    scheduled: usize,
    attempted: usize,
    decoded: usize,
    errors: usize,
    worker_elapsed: Duration,
    commit_elapsed: Duration,
) {
    if !starmap_telemetry::enabled() {
        return;
    }
    let outcome = if errors > 0 {
        "errors"
    } else if decoded == 0 {
        "empty"
    } else {
        "decoded"
    };
    tracing::info!(
        target: "perf::audio_start",
        module = "preview_audition_warm",
        event = "preview_audition.warm_finished",
        outcome,
        scheduled,
        attempted,
        decoded,
        errors,
        worker_elapsed_ms = worker_elapsed.as_secs_f64() * 1000.0,
        commit_elapsed_ms = commit_elapsed.as_secs_f64() * 1000.0,
        "Preview audition warm finished"
    );
}

pub(super) fn record_preview_audition_warm_phase_profile(
    display_mode: SampleBrowserDisplayMode,
    outcome: &'static str,
    reason: Option<&'static str>,
    summary: PreviewAuditionWarmPhaseSummary,
    total_elapsed: Duration,
    plan_elapsed: Duration,
    reservation_elapsed: Duration,
    task_schedule_elapsed: Duration,
) {
    let telemetry_enabled = starmap_telemetry::enabled();
    let slow = total_elapsed >= PREVIEW_AUDITION_WARM_PHASE_PROFILE_THRESHOLD;
    if !slow && !telemetry_enabled {
        return;
    }
    if slow {
        tracing::warn!(
            target: "wavecrate::debug::ui_frame",
            module = "preview_audition_warm",
            event = "preview_audition.warm_phase_profile",
            display_mode = sample_browser_display_mode_str(display_mode),
            outcome,
            reason = reason.unwrap_or(""),
            scheduled = summary.scheduled,
            inspected = summary.inspected,
            candidates = summary.candidates,
            eligible = summary.eligible,
            starmap_cells = summary.starmap_cells,
            starmap_visited_cells = summary.starmap_visited_cells,
            starmap_remaining_budget = summary.starmap_remaining_budget,
            list_remaining_budget = summary.list_remaining_budget,
            total_elapsed_ms = total_elapsed.as_secs_f64() * 1000.0,
            plan_elapsed_ms = plan_elapsed.as_secs_f64() * 1000.0,
            reservation_elapsed_ms = reservation_elapsed.as_secs_f64() * 1000.0,
            task_schedule_elapsed_ms = task_schedule_elapsed.as_secs_f64() * 1000.0,
            "Slow preview audition warm phase"
        );
    } else {
        tracing::info!(
            target: "perf::audio_start",
            module = "preview_audition_warm",
            event = "preview_audition.warm_phase_profile",
            display_mode = sample_browser_display_mode_str(display_mode),
            outcome,
            reason = reason.unwrap_or(""),
            scheduled = summary.scheduled,
            inspected = summary.inspected,
            candidates = summary.candidates,
            eligible = summary.eligible,
            starmap_cells = summary.starmap_cells,
            starmap_visited_cells = summary.starmap_visited_cells,
            starmap_remaining_budget = summary.starmap_remaining_budget,
            list_remaining_budget = summary.list_remaining_budget,
            total_elapsed_ms = total_elapsed.as_secs_f64() * 1000.0,
            plan_elapsed_ms = plan_elapsed.as_secs_f64() * 1000.0,
            reservation_elapsed_ms = reservation_elapsed.as_secs_f64() * 1000.0,
            task_schedule_elapsed_ms = task_schedule_elapsed.as_secs_f64() * 1000.0,
            "Preview audition warm phase profile"
        );
    }
}
