# Sempal Improvement Audit Plan

Last updated: 2026-04-19 by Codex
Scope: repo-wide Phase 1 evidence-driven audit for the current live tree
Status: Phase 1 complete; waiting for explicit implementation confirmation

## Context Snapshot

- Project purpose: audio sample triage tool built with Rust. Explicitly documented in `README.md`.
- Maturity: active alpha with strong local guardrails, ongoing `app_core`/Radiant migration, and recent performance follow-up work. Strongly implied by `README.md`, `docs/ARCHITECTURE.md`, `AGENTS.md`, and `MEMORY.md`.
- Primary stack: Rust workspace with a root app crate plus `sempal-analysis`, `sempal-library`, `sempal-scan`, installer/updater apps, and vendored `radiant`. Explicitly documented in `Cargo.toml` and `README.md`.
- Canonical validation commands: `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1` on Windows. Explicitly documented in `README.md`, `docs/TEST.md`, and `AGENTS.md`.
- Architectural boundaries: `src/app_core/**` owns host-facing projections, `src/app/**` still owns legacy controller/model logic, and `vendor/radiant/**` owns reusable GUI/runtime behavior. Explicitly documented in `docs/ARCHITECTURE.md`.
- Audit note: the current live tree is dirty and includes a large uncommitted script/docs reshuffle. Findings below are grounded in the present workspace contents unless stated otherwise.

## Ordered Backlog

### [ ] OPT-52 — Sempal: restore top-level validation entrypoints and guardrail path consistency after the scripts/internal migration
- Classification: Developer-experience improvement
- Primary label: `Improvement`
- Confidence: High
- ROI: High
- Effort: Medium
- Why it matters:
  The documented validation workflow does not resolve in the current live tree, so routine development and agent validation start from a broken contract.
- Evidence:
  - `README.md`, `docs/TEST.md`, and `AGENTS.md` all point to top-level wrappers such as `scripts/devcheck.ps1`, `scripts/ci_agent.ps1`, `scripts/ci_quick.ps1`, `scripts/ci_local.ps1`, `scripts/run_sandbox.ps1`, `scripts/clean_sandbox.ps1`, `scripts/latest_log.ps1`, and `scripts/bug_bundle.ps1`.
  - `scripts/README.md` also lists those top-level entrypoints as the public workflow surface.
  - The live tree instead contains `scripts/internal/ci/*.ps1` and `scripts/internal/run/*.ps1`, while the documented top-level wrapper files are absent.
  - `scripts/internal/check/check_file_size_budget.ps1` still hardcodes `scripts/check/allowlists/file_size_budget_allowlist.txt`, but the live allowlist is under `scripts/internal/check/allowlists/file_size_budget_allowlist.txt`.
  - `scripts/internal/ci/devcheck.ps1` help text still points users to `scripts/devcheck.ps1`, `scripts/ci_quick.ps1`, and `scripts/ci_local.ps1`.
- Recommended change:
  Decide the public script contract, then make docs, top-level wrappers, and internal help strings consistent with it. Repair legacy hardcoded `scripts/check/**` references inside guardrail scripts.
- Expected impact:
  Restores a reliable local validation path and reduces false debugging churn around “missing script” failures.
- Risks or tradeoffs:
  If the migration is intentionally incomplete, preserve the intended final structure rather than reintroducing accidental duplication.
- Dependencies:
  None.
- Suggested validation:
  Confirm every documented Windows wrapper path resolves and run at least the documented `devcheck` and `ci_agent` entrypoints successfully.
- Product clarification required:
  No.

### [ ] OPT-50 — Split controller background-job DTOs into lane-specific modules
- Classification: Architecture improvement
- Primary label: reuse existing issue
- Confidence: High
- ROI: Medium-High
- Effort: Medium
- Why it matters:
  Controller job DTOs currently collect many unrelated lanes in one module, which makes follow-on cleanup in the file-mutation lane harder to stage safely.
- Evidence:
  - `src/app/controller/jobs/messages.rs` is 597 lines and contains a wide cross-section of controller job messages and payload DTOs.
  - The open Linear issue title matches the observed repository pressure directly.
- Recommended change:
  Reuse `OPT-50` instead of creating a duplicate; treat it as the prerequisite narrowing pass for downstream file-op cleanup.
- Expected impact:
  Shrinks the DTO surface and gives later lane-focused refactors cleaner seams.
- Risks or tradeoffs:
  The split should preserve completion routing and worker/result matching behavior.
- Dependencies:
  None.
- Suggested validation:
  Run the controller job routing tests and the agent-safe validation lane after the split.
- Product clarification required:
  No.

### [ ] OPT-54 — Sempal: split file-op apply flows from folder mutation execution and recovery paths
- Classification: Refactor / cleanup
- Primary label: `cleanup`
- Confidence: High
- ROI: Medium-High
- Effort: Medium
- Why it matters:
  One destructive-workflow lane is spread across oversized modules that mix DTOs, background execution, result application, undo, recovery, and browser mutation helpers.
- Evidence:
  - `src/app/controller/ui/file_ops.rs` is 603 lines and applies many unrelated file-op result families.
  - `src/app/controller/library/source_folders/actions/rename_move_delete.rs` is 573 lines and mixes entrypoints, background jobs, recovery logic, focus remapping, undo construction, and test-only failure injection.
  - `src/app/controller/library/browser_controller/helpers.rs` is 513 lines and mixes normalization, delete/rename flows, focus planning, and rename helpers.
  - All three files sit outside the file-size budget allowlist in `scripts/internal/check/allowlists/file_size_budget_allowlist.txt`.
- Recommended change:
  After `OPT-50`, split file-op apply handlers, folder mutation executors/recovery helpers, and browser sample mutation helpers into lane-focused modules with tests kept near the resulting ownership boundaries.
- Expected impact:
  Makes destructive-flow fixes safer and easier to audit.
- Risks or tradeoffs:
  This lane is correctness-sensitive; recovery order and optimistic-state semantics must remain unchanged.
- Dependencies:
  - `OPT-50`
- Suggested validation:
  Run folder delete/rename recovery tests, browser sample mutation tests, and the agent-safe validation lane.
- Product clarification required:
  No.

### [ ] OPT-53 — Sempal: split browser visible-row pipeline stages into focused filter/query/similarity modules
- Classification: Refactor / cleanup
- Primary label: `cleanup`
- Confidence: High
- ROI: Medium-High
- Effort: Medium
- Why it matters:
  Browser filter/query/similarity staging is concentrated in a single non-allowlisted module, and nearby stale-row bugs have already shown this area is fragile.
- Evidence:
  - `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` is 496 lines.
  - `build_visible_rows_with_now` coordinates duplicate cleanup, folder acceptance, filter fingerprints, query dispatch, similarity dispatch, and visible-row remapping.
  - The file also owns `ensure_filtered_stage`, `retained_filter_only_rows`, `ensure_sorted_stage_for_similar`, `ensure_sorted_stage_for_query`, and `ensure_sorted_stage_for_filter_only`.
  - Recent related issues exist nearby: `OPT-39`, `OPT-40`, and `OPT-41`.
- Recommended change:
  Split retained/filter-stage logic, query/similarity sort-stage logic, and final visible-row assembly into separate focused modules while preserving existing cache-key semantics exactly.
- Expected impact:
  Makes browser row/cache bugs easier to localize and reduces the cost of future filter/query work.
- Risks or tradeoffs:
  Accidental fingerprint drift could reintroduce stale-row defects if the split changes cache semantics.
- Dependencies:
  None.
- Suggested validation:
  Re-run the browser pipeline suites and stale-row regression coverage around `OPT-39`, `OPT-40`, and `OPT-41`.
- Product clarification required:
  No.

## Open Questions / Missing Definitions

### [!] Is the current script-entrypoint migration meant to remove the top-level wrappers permanently?
- Evidence:
  The live tree removed the documented top-level wrappers, but docs and internal help still treat them as canonical.
- Why it matters:
  The fix for `OPT-52` depends on whether the desired end state is “restore shims” or “rewrite every public command reference.”
- Affected files or modules:
  `README.md`, `AGENTS.md`, `docs/TEST.md`, `scripts/README.md`, `scripts/internal/ci/devcheck.ps1`, `scripts/internal/check/check_file_size_budget.ps1`
- Risk if guessed incorrectly:
  We could either reintroduce wrapper duplication the repo is trying to retire or update docs toward paths that were meant to stay internal-only.
- Most conservative provisional assumption:
  Keep the user-facing top-level wrapper contract intact unless maintainers explicitly want the public commands to move.

## Rejected Ideas

### [-] Split `ControllerRuntimeState` immediately as a top backlog item
- Why it was considered:
  `src/app/controller/state/runtime/mod.rs` is 451 lines and mixes many pending/deferred controller concerns in one state object.
- Why it was rejected:
  The evidence supports architectural pressure, but the work is broader and less execution-ready than the script consistency fix and the already-clustered browser/file-op cleanup lanes. It also overlaps existing migration issues like `OPT-34` and `OPT-44`.
- Missing evidence:
  I did not find a narrowly bounded repository symptom or existing test cluster that would let this be shaped into a smaller, safer first-pass ticket today.

## Audit Summary

- Total backlog items: 4
- Newly created issues: 3 (`OPT-52`, `OPT-53`, `OPT-54`)
- Reused issues: 1 (`OPT-50`)
- Duplicate avoidance:
  Reused `OPT-50` instead of creating a second job-DTO split ticket.
