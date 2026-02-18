
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
- `run_sandbox` is safer than direct runs but can still write `.sempal_samples.db` in a source tree if launched there.
- Default run/worktree flow still mixes review, CI, and interactive execution steps.
- Runtime diagnostics are primarily log-based and human-readable, with limited machine-readable status reporting.
- `docs/QUALITY_SCORE.md` and the plans area track gaps; they are not yet hard enforcement.

### Recommended start sequence for new agents
- Read `docs/README.md`, then `docs/FEATURE_CHECKLIST.md`, then `docs/ARCHITECTURE.md`.
- Confirm environment/tooling with `bash scripts/doctor.sh`.
- Verify current plan and debt context in `docs/plans/*` and `docs/QUALITY_SCORE.md`.
- Run `bash scripts/ci_local.sh` before and after edits.
