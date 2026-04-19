# Developer Docs

This directory now keeps a deliberately small set of canonical developer
documents. Anything that is narrow, historical, or fast-moving should live in
`tmp/` or `docs/plans/` instead of growing the long-term docs set again.

Machine-consumed check allowlists do not live here anymore. They now live under
`scripts/check/allowlists/` so `docs/` stays human-facing.

## Canonical docs

- `docs/ARCHITECTURE.md`
  - product principles, ownership boundaries, and the Radiant compatibility
    boundary
- `docs/ENV_VARS.md`
  - environment variable reference and safety notes
- `docs/TEST.md`
  - development workflow, validation gates, and test suite map
- `docs/SYSTEMS.md`
  - runtime contracts, recovery rules, automation surfaces, and data formats
- `docs/TROUBLESHOOTING.md`
  - common failure modes, diagnostics, and guardrail-change workflow

## Live operational files

- `AGENTS.md`
  - wake-up portal and current mission
- `MEMORY.md`
  - present-tense session snapshot
- `docs/plans/index.md`
  - stable map for active plan artifacts and templates
- `docs/plans/TEMPLATE_execution_plan.md`
  - reusable template for execution plans
- `docs/plans/TEMPLATE_investigation.md`
  - reusable template for investigation writeups
- `docs/plans/active/todo.md`
  - short ordered queue for the active lane
- `tmp/improvement_audit_plan.md`
  - canonical source for the current audit lane status and execution order
- `tmp/perf_plan.md`
  - runtime-performance backlog and execution record for the live tree
- `tmp/cleanup_plan.md`
  - parked cleanup backlog
- `tmp/bug_audit_plan.md`
  - latest bug-audit snapshot

## Default workflow

1. Run request preflight.
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
   - macOS/Linux/WSL:
     `bash scripts/run_agent_request.sh`
2. Use `docs/TEST.md` for the right validation lane.
3. Use `docs/TROUBLESHOOTING.md` when a guardrail or environment check fails.
4. Keep changes small, update the canonical doc that owns the changed behavior,
   and avoid creating one-off docs unless the information truly needs to live
   separately.

## Principles for this folder

- Prefer a few strong documents over many narrow notes.
- Prefer current contracts over historical execution diaries.
- Put detailed plans in `docs/plans/` or `tmp/`, not in the canonical docs.
- Delete stale docs instead of keeping them as archaeological layers.
