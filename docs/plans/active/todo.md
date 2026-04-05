# Active TODO (Agent Handoff Queue)

Last updated (local): 2026-04-05T14:57:00+02:00
Owner: Codex agent sessions

Purpose:
- Keep this file short, ordered, and action-focused.
- Track only immediate next tasks for the active execution lane.
- Keep deep rationale and ranked backlog detail in `tmp/perf_plan.md`.

## Current lane

- The active lane is the reopened runtime performance audit backlog for the current live tree.
- `tmp/perf_plan.md` is the live source of truth for the ROI-ranked performance backlog rebuilt on 2026-04-05.
- Phase 2 is in progress. Item 1 is complete (`bd2b6a57`) and item 2 is next.
- Measurement caveat: the current perf guard still measures the controller-mode `project_native_app_model` path for most GUI scenarios, while the shipped runtime goes through retained `SempalNativeBridge`; the Windows PowerShell perf guard also still lacks startup capture.
- `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` remain parked while this performance lane is active.

## Next tasks (ordered)

1. Execute item 2 from `tmp/perf_plan.md` next and keep the remaining backlog in strict ROI order.
2. After each completed item, update `tmp/perf_plan.md`, validate, commit, and push before moving on.
3. Keep `tmp/improvement_audit_plan.md` and `tmp/cleanup_plan.md` dormant unless the user explicitly reopens those lanes.
4. Keep using the PowerShell validation wrappers for future Windows sessions unless the user explicitly overrides that rule.
