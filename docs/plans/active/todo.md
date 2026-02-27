# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-27T12:10:33Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).
- File-size debt burn-down for top 5 Rust hotspots (behavior-preserving splits).

## Next tasks (ordered)

1. Reduce compositor-run warning drift in browser-heavy scenarios
   (`hover_latency`, `wheel_latency`, `browser_filter_churn_latency`) using
   the latest 7-run perf-guard evidence.
2. Root-cause projection-stage spikes in waveform interaction outliers under
   immediate-preview-on runs before revisiting immediate-apply scope.
3. Maintain handoff hygiene on every milestone commit:
   update `AGENTS.md`, `MEMORY.md`, and this queue in the same change set.
4. Execute next no-behavior `jobs.rs` slice:
   extract issue gateway/token worker runners into a focused `jobs/` submodule.
5. Execute first `browser_search_worker.rs` split slice:
   isolate telemetry counters + cadence emission helpers into a focused submodule.
