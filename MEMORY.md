# Agent Memory

Last Updated: 2026-02-20T11:40:20Z
Updated By: Codex

## Purpose

- Keep session handoff durable for agents and automation.
- Record the latest objective and current execution state.

## Current Session (2026-02-20 UTC)

- I am finishing Phase 4 closure work for runtime performance guardrails in
  `docs/plans/active/runtime_performance_exec_plan.md`.
- I added a wheel-stability workflow (`scripts/run_perf_wheel_stability.sh`
  plus PowerShell wrapper) that collects repeated perf windows and evaluates
  hard-fail promotion readiness into machine-readable JSON.
- I promoted `wheel_latency` to a conservative default hard-fail threshold in
  `scripts/run_perf_guard.sh` (`SEMPAL_PERF_FAIL_P95_US_WHEEL=30000`) and kept
  stage-attribution reporting enabled.
- I updated guardrails/docs (`docs/ENV_VARS.md`, `docs/performance_qa.md`,
  `docs/INDEX.md`, and script guardrail checks) to reflect the new workflow.
- `bash scripts/ci_local.sh` is green and I am preparing commit/push for this
  milestone.

## Work Notes

- Latest pushed commits:
  - `vendor/radiant`: `cb9999b` (`perf(native_vello): intern text layout keys and atom cache`)
  - `sempal`: `13b01fae` (`perf(bench): add stage-attributed interaction metrics`)
- Pending commit (not yet pushed): wheel-stability evidence workflow,
  conservative wheel hard-fail promotion, and supporting docs/guardrails.
