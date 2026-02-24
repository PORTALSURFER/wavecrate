# Developer index (invariants, allowlists, and remediation)

If you're an agent or a new contributor: start in `docs/README.md`, then come back here when a CI/local check fails.

## Where checks run

- Local CI parity: `scripts/ci_local.{sh,ps1}`
- CI: `.github/workflows/ci.yml`

## Tooling and scripts (recommended workflow chain)

These are the default “don’t guess, don’t grep” entrypoints for most work:

1. Bootstrap tooling + pinned toolchain:
   - `bash scripts/bootstrap.sh` (or `scripts/bootstrap.sh --verify-only`)
   - `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1 --verify-only`
2. Environment sanity checks:
   - `bash scripts/doctor.sh`
   - `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
3. Mandatory agent request preflight:
   - `bash scripts/run_agent_request.sh`
   - `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
   - `bash scripts/run_agent_preflight.sh` (lightweight version; no full `ci_local`)
   - `bash scripts/install_agent_preflight_hooks.sh` (optional auto-hook install)
4. Safe local run (isolated config/logs):
   - `bash scripts/run_sandbox.sh --`
   - `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1 --`
5. Find and tail the newest log:
   - `bash scripts/latest_log.sh`
   - `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
6. Create a bug bundle (logs + config + versions):
   - `bash scripts/bug_bundle.sh`
   - `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`
7. Reset sandbox state (fresh start):
   - `bash scripts/clean_sandbox.sh`
   - `powershell -ExecutionPolicy Bypass -File scripts/clean_sandbox.ps1`

## Checks glossary (diff-aware vs full scan)

Agents should optimize for diff-aware checks during iteration, and reserve full scans for periodic cleanup.

- Diff-aware checks (default behavior):
  - Operate on git diff ranges and/or staged/unstaged changes.
  - Prevent introducing new violations without forcing immediate cleanup of legacy debt.
  - `scripts/check_file_size_budget.*` (diff-aware by default; full scan via `--all`)
  - `scripts/check_manual_docs_scope.*`
  - `scripts/check_legacy_app_coupling.*`
  - `scripts/check_rust_taste_invariants.*`
  - `scripts/check_rust_no_todos.*`
  - `scripts/check_rust_public_docs.*`
  - `scripts/check_app_core_dependency_boundary.*`
  - `scripts/check_markdown_links.*`
- Full-scan checks:
  - Scan a fixed scope every time (usually still fast, but not “only the diff”).
  - `scripts/check_migration_boundary.*` (scans `src/app_core/**`)
  - `scripts/check_docs_index.*` (scans `docs/README.md` references)
  - `scripts/check_codeowners_coverage.*` (scans `.github/CODEOWNERS` for bucket coverage)
  - `scripts/check_rust_dead_deps_advisory.sh` (advisory lane for `cargo machete` + optional `cargo udeps`)

## When a check fires (what to do)

1. Prefer fixing the underlying issue (refactor/move code/add docs) over adding an exception.
2. If a check has an allowlist, only use it as a last resort and add a short justification comment in the allowlist file.
3. If you intentionally changed repo structure or docs topology, update:
   - `docs/README.md` (landing page)
   - `scripts/check_docs_index.*` (required references)
   - `docs/INDEX.md` (this file)

## Invariant checks (scripts/check_*.{sh,ps1})

| Check | What it enforces | How to fix when it fails |
| --- | --- | --- |
| `scripts/check_migration_boundary.sh` | `src/app_core/**` must not reference `crate::app::` except `src/app_core/app_api.rs`. | Move legacy app dependencies behind `app_api` or move code into the legacy layer. |
| `scripts/check_migration_boundary.ps1` | PowerShell equivalent of the migration-boundary check. | Same remediation as the bash version. |
| `scripts/check_file_size_budget.sh` | Rust files must stay under the file-size budget (default `400` LOC), diff-aware by default. | Split the module, extract submodules, or reduce responsibilities; last resort: allowlist. |
| `scripts/check_file_size_budget.ps1` | PowerShell equivalent of the file-size budget check. | Same remediation as the bash version. |
| `scripts/check_script_guardrails.sh` | Key shell scripts must stay syntax valid and pass fixture checks for matching logic. | Keep script syntax and fixture assertions green; ensure regex matching and argument parsing are fixture-covered. |
| `scripts/check_script_guardrails.ps1` | PowerShell wrapper around the script guardrails check. | Same remediation as the bash version. |
| `scripts/run_agent_request.sh` | Refreshes `MEMORY.md`, runs mandatory guardrails, and executes full local CI. | Run at session start; do not continue if this check fails. |
| `scripts/run_agent_request.ps1` | PowerShell equivalent of the agent preflight + local CI entrypoint. | Same remediation as the bash version. |
| `scripts/run_perf_guard.sh` | Runs deterministic runtime interaction benchmarks and evaluates warn/fail thresholds (including stage attribution where available). | Use for local perf regression checks and CI parity before push. |
| `scripts/run_perf_guard.ps1` | PowerShell wrapper for `scripts/run_perf_guard.sh`. | Same remediation as the bash version. |
| `scripts/run_perf_wheel_stability.sh` | Collects repeated wheel-latency perf windows and evaluates hard-fail promotion readiness. | Use when deciding whether wheel fail thresholds are stable enough to tighten. |
| `scripts/run_perf_wheel_stability.ps1` | PowerShell wrapper for `scripts/run_perf_wheel_stability.sh`. | Same remediation as the bash version. |
| `scripts/run_agent_preflight.sh` | Runs mandatory preflight checks (`run_agent_ci_checks.sh`) with configurable MEMORY refresh behavior. | Use from branch/pull entrypoints when you need the mandatory checks without full local CI. |
| `scripts/install_agent_preflight_hooks.sh` | Installs git hooks that auto-run `run_agent_preflight.sh` after merge/checkout. | Install in local workspaces that should enforce preflight on every pull/branch switch. |
| `scripts/check_quality_score_drift.sh` | Ensures `docs/QUALITY_SCORE.md` reflects current high-visibility guardrail status (file-size budget + Rust taste invariants) and flags drift. | Update the score row in `docs/QUALITY_SCORE.md` whenever this check degrades or recovers. |
| `scripts/check_quality_score_drift.ps1` | PowerShell wrapper for score-drift enforcement check. | Same remediation as the bash version, then update `docs/QUALITY_SCORE.md` to keep handoff context accurate. |
| `scripts/check_manual_docs_scope.sh` | `manual/` is user docs only; new/changed files in `manual/` must be allowlisted (site assets + user docs + redirect stubs). | Move developer docs into `docs/`; keep `manual/` for user content and site assets. |
| `scripts/check_manual_docs_scope.ps1` | PowerShell equivalent of the manual-scope check. | Same remediation as the bash version. |
| `scripts/check_legacy_app_coupling.sh` | Prevent new `crate::app` coupling from non-legacy codepaths (diff-aware; skips `src/app/**` and `src/legacy_runtime/**`). | Move code into legacy paths, route through `app_core`, or isolate behind a boundary; last resort: allowlist. |
| `scripts/check_legacy_app_coupling.ps1` | PowerShell equivalent of the legacy-coupling check. | Same remediation as the bash version. |
| `scripts/check_rust_taste_invariants.sh` | Disallow added `dbg!`, `println!`, `.unwrap()`, `.expect()` in non-test Rust (diff-aware). | Use `tracing`, propagate errors, or confine patterns to tests/benches; last resort: allowlist. |
| `scripts/check_rust_taste_invariants.ps1` | PowerShell equivalent of the Rust taste invariants check. | Same remediation as the bash version. |
| `scripts/check_rust_no_todos.sh` | Disallow added `TODO`/`FIXME` markers in non-test Rust (diff-aware). | Implement the fix now, file an issue instead, or capture the plan in `docs/plans/` instead of in-code TODOs; last resort: allowlist. |
| `scripts/check_rust_no_todos.ps1` | PowerShell equivalent of the no-TODO/FIXME check. | Same remediation as the bash version. |
| `scripts/check_rust_dead_deps_advisory.sh` | Advisory sweep for unused dependencies/dead code using `cargo machete` and optional `cargo udeps`; non-blocking by default. | Install missing tools (`--install-missing`), review findings, and promote to `--strict` only after false positives are tuned out. |
| `scripts/check_rust_public_docs.sh` | Newly added `pub` Rust items must have nearby doc comments (`///` or `#[doc = ...]`), diff-aware. | Add `///` docs describing what/why/constraints; include examples if non-obvious; last resort: allowlist. |
| `scripts/check_rust_public_docs.ps1` | PowerShell equivalent of the public-docs check. | Same remediation as the bash version. |
| `scripts/check_app_core_dependency_boundary.sh` | `src/app_core/**` must not take new dependencies on `crate::legacy_runtime::`, `crate::gui_app::`, `crate::gui_runtime::` (diff-aware). | Move UI/runtime coupling into the appropriate layer (`src/gui_app`, `src/gui_runtime`, `src/legacy_runtime`) or invert the dependency. |
| `scripts/check_app_core_dependency_boundary.ps1` | PowerShell equivalent of the app_core dependency boundary check. | Same remediation as the bash version. |
| `scripts/check_docs_index.sh` | `docs/README.md` must reference required docs and all referenced `docs/*.md` must exist. | Update `docs/README.md` to include required links and fix broken references. |
| `scripts/check_docs_index.ps1` | PowerShell equivalent of the docs index check. | Same remediation as the bash version. |
| `scripts/check_codeowners_coverage.sh` | `.github/CODEOWNERS` must contain entries for the high-level ownership buckets. | If ownership boundaries changed, update `docs/ARCHITECTURE.md` and then update `.github/CODEOWNERS` to match (see “Ownership and CODEOWNERS”). |
| `scripts/check_codeowners_coverage.ps1` | PowerShell equivalent of the CODEOWNERS coverage check. | Same remediation as the bash version. |
| `scripts/check_markdown_links.sh` | Changed Markdown files must not introduce broken local file links (diff-aware). | Fix the link target, update the path, or delete stale references. |
| `scripts/check_markdown_links.ps1` | PowerShell equivalent of the Markdown link check. | Same remediation as the bash version. |

## Knowledge linter (wrapper)

- `scripts/knowledge_lint.sh` runs `scripts/check_docs_index.sh`, `scripts/check_codeowners_coverage.sh`, and `scripts/check_markdown_links.sh`.
- `scripts/knowledge_lint.ps1` runs the PowerShell equivalents.

## Allowlists (docs/*allowlist*.txt)

| Allowlist | Used by | When to use |
| --- | --- | --- |
| `docs/file_size_budget_allowlist.txt` | `scripts/check_file_size_budget.*` | Only for known legacy oversized files during transition; prefer splitting files and removing entries over time. |
| `docs/legacy_app_coupling_allowlist.txt` | `scripts/check_legacy_app_coupling.*` | Rare transitional shims only; prefer refactors that remove coupling. |
| `docs/rust_taste_invariants_allowlist.txt` | `scripts/check_rust_taste_invariants.*` | Last resort when a non-test file must temporarily contain the forbidden patterns. |
| `docs/app_core_dependency_boundary_allowlist.txt` | `scripts/check_app_core_dependency_boundary.*` | Last resort for temporary boundary violations while migrating. |
| `docs/rust_no_todos_allowlist.txt` | `scripts/check_rust_no_todos.*` | Last resort for exceptional cases; prefer issues or `docs/plans/`. |
| `docs/rust_public_docs_allowlist.txt` | `scripts/check_rust_public_docs.*` | Last resort if a public item intentionally must remain undocumented (rare). |
