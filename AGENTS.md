
# Agent entrypoints

Read first:

- `docs/README.md` (developer docs landing page)
- `docs/FEATURE_CHECKLIST.md` (safe path for changes)
- `docs/ARCHITECTURE.md` (ownership + where changes should go)
- `docs/ENV_VARS.md` (env var reference, including ASIO build notes)
- `docs/design_principles.md` (architecture goals/constraints)

Golden path commands (CI parity + diagnostics):

- Bootstrap tooling + pinned toolchain: `bash scripts/bootstrap.sh` or `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
- CI parity checks: `bash scripts/ci_local.sh` or `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Safe local run (isolated config/logs): `bash scripts/run_sandbox.sh --` or `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1`
- Clean sandbox state: `bash scripts/clean_sandbox.sh` or `powershell -ExecutionPolicy Bypass -File scripts/clean_sandbox.ps1`
- Environment sanity checks: `bash scripts/doctor.sh` or `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
- Tail newest log: `bash scripts/latest_log.sh` or `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
- Create bug bundle: `bash scripts/bug_bundle.sh` or `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`

Rules:

- After any code change, create a commit and push it.
  If your environment requires explicit approval for git operations, ask for confirmation and include the intended commit message.
- Before pushing, run `scripts/ci_local.sh` after every change and do not push until it is green.
  If any check fails, fix issues and rerun until it passes.
  If you cannot run scripts in your environment, stop and resolve before pushing.

- Mandatory agent request preflight:
  - On each agent request/session, run `bash scripts/run_agent_request.sh` (or
    `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`).
- `scripts/run_agent_preflight.sh` is the lightweight preflight primitive and is
  used by `run_agent_request.sh`. Automatic enforcement after `git pull` or
  branch checkout is installed by `bash scripts/bootstrap.sh` by default and uses
  `bash scripts/install_agent_preflight_hooks.sh` to enforce the lightweight preflight hook.
  - Hook environment overrides: `AGENT_PREFLIGHT_UPDATER` (default `Codex`),
    `AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS` (default `1`), and
    `SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1` to skip hook execution and
    `SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK_INSTALL=1` to skip bootstrap installation.
  - Agent-CI memory guardrails (used by `run_agent_ci_checks.sh` and `ci_local.sh`):
    `AGENT_CI_REQUIRED_UPDATER` (default: unset) and
    `AGENT_CI_MEMORY_MAX_AGE_HOURS` (default: `24`).
  - If this check fails, do not proceed with code changes until it is green.
- `scripts/run_sandbox.sh` and `scripts/run_sandbox.ps1` default to read-only source DB mode (`--write-db` required).
- `manual/` is user-facing documentation only. Developer docs belong in `docs/`.

## Current Agent Context

### Where we are now
- Repository: `/home/uhx/dev/sempal`
- Product: Sempal (sample manager app)
- Work mode: agent-facing review + onboarding hardening
- Current milestone: applying the second review pass to increase agent-friendliness and reduce hidden side effects.

### What is happening right now
- Reviewing and documenting high-friction areas in docs, scripts, and runtime behavior before making broad refactors.
- No active feature or behavior changes are committed in this pass.
- The immediate goal is to improve handoff quality so the next agent can continue without re-discovering decisions.

### Current known constraints / hazards
- `run_sandbox` defaults to no writes in source trees (`--write-db` and `--allow-user-library-db-write` are explicit overrides).
- Default run/worktree flow still mixes review, CI, and interactive execution steps.
- Runtime diagnostics are primarily log-based and human-readable, with limited machine-readable status reporting.
- `docs/QUALITY_SCORE.md` includes a score row for agent-facing guardrails that is now enforced by `scripts/check_quality_score_drift.sh` in local CI and GitHub CI.

### Recommended start sequence for new agents
- Read `docs/README.md`, then `docs/FEATURE_CHECKLIST.md`, then `docs/ARCHITECTURE.md`.
- Confirm environment/tooling with `bash scripts/doctor.sh`.
- Verify current plan and debt context in `docs/plans/*` and `docs/QUALITY_SCORE.md`.
- Run `bash scripts/ci_local.sh` before and after edits.
