# Active TODO (Agent Handoff Queue)

Last updated (UTC): 2026-02-21T09:58:56Z
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and architecture notes in
  `docs/plans/active/runtime_performance_exec_plan.md`.

## Current lane

- Runtime responsiveness/performance redesign (Xilem-inspired retained/incremental update path).

## Next tasks (ordered)

1. Add bridge profiler counters/reporting for static-scene rebuild causes so
   action-class attribution is visible in perf captures.
2. Audit browser viewport row projection for additional low-allocation wins
   (selected-row/path lookup refresh and string/path reuse).
3. Tighten interactive slider drag paths (volume and similar controls) to
   guarantee immediate visual updates and no release-only redraw dependency.
4. Keep `MEMORY.md` and this queue updated in every perf milestone commit.

## Done recently

- Completed ROI item #1: switched waveform multi-step zoom to single-pass math with regression coverage.
- Completed Phase 7 item 1 foundation: tightened radiant invalidation scope routing so
  high-frequency browser/search/prompt actions use model+overlay invalidation.
- Completed Phase 7 static rebuild scope follow-up: model-refresh static
  rebuilds now stay dirty-mask-driven unless explicit static invalidation is
  requested, with focused radiant tests.
- Completed browser projection micro-optimization: removed visible-row window
  copy in `project_browser_rows_model` and switched to direct visible index
  reads with bounded pre-allocation.
- Completed Phase 7 waveform cache-hit improvement: quantized waveform view
  metadata matching and stabilized texture-width bucketing.
- Completed Phase 7 delta pan milestone: added waveform translation reuse
  (shift + edge patch render) with adjacent-pan regression coverage.
- Completed deferred waveform seek-commit milestone: native seek interactions
  now queue/debounce replay seeks, reducing waveform interaction apply-stage
  cost with regression coverage.
