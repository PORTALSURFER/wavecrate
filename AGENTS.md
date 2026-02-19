# Agent Wake-Up Portal

Purpose: this file is a minimal orientation map for stateless agents.
Do not store deep specs or long plans here. Put details in `docs/` and link to
them.

## Current Mission
- Repository: `/home/uhx/dev/sempal`
- Product: Sempal (sample manager app)
- Active program: runtime responsiveness/performance redesign inspired by
  Xilem-style scoped invalidation and cache reuse.
- Source of truth for active execution: `docs/plans/active/plan.md` under
  `Runtime Performance Redesign (Multi-Day) Checklist`.

## Wake-Up Sequence
1. Run mandatory preflight: `bash scripts/run_agent_request.sh`
2. Read, in order:
   - `docs/README.md`
   - `docs/FEATURE_CHECKLIST.md`
   - `docs/ARCHITECTURE.md`
   - `docs/design_principles.md`
   - `docs/plans/active/plan.md`
3. Confirm environment if needed: `bash scripts/doctor.sh`
4. Before and after edits: `bash scripts/ci_local.sh`

## Non-Negotiable Workflow Rules
- After any code change: run `scripts/ci_local.sh`, commit, and push.
- Do not push unless `scripts/ci_local.sh` is green.
- If CI fails: fix in place and rerun until green.
- If git/network permissions are restricted: request approval with the intended
  command/message.

## Golden Commands
- Bootstrap: `bash scripts/bootstrap.sh`
- Local CI parity: `bash scripts/ci_local.sh`
- Safe run: `bash scripts/run_sandbox.sh --`
- Clean sandbox state: `bash scripts/clean_sandbox.sh`
- Diagnostics: `bash scripts/doctor.sh`
- Latest log: `bash scripts/latest_log.sh`
- Bug bundle: `bash scripts/bug_bundle.sh`

## Guardrails and Constraints
- `scripts/run_sandbox.sh` defaults to read-only source DB mode (`--write-db`
  required for writes).
- `manual/` is user-facing docs only; developer docs belong in `docs/`.
- Agent preflight hooks are installed by bootstrap and enforced by
  `scripts/run_agent_preflight.sh`.
- Memory freshness/quality score checks are enforced via
  `scripts/run_agent_ci_checks.sh` and `scripts/check_quality_score_drift.sh`.

## Memory Hygiene
- Keep `AGENTS.md` short and current.
- When priorities change, update this file first, then update linked plan/docs.
- Store large context in dedicated docs and reference them here with short
  labels.
