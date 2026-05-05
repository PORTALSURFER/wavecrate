# Developer Docs

This directory now keeps a deliberately small set of canonical developer
documents. Planning and backlog state now live in Linear rather than Markdown
plan files. Anything narrow, historical, or investigatory should live in `tmp/`
only when a durable canonical doc is not the right fit.

Machine-consumed check allowlists do not live here anymore. They now live under
`scripts/internal/check/allowlists/` so `docs/` stays human-facing.

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
  - repo-specific workflow rules, validation entrypoints, branch policy, and
    repo-to-Linear project mapping
- Planning and backlog
  - live in Linear project `Sempal` under team `PORTALSURFER`
- `tmp/database_system_audit_plan.md`
  - database-system audit notes and follow-up context
- `tmp/source_runtime_test_isolation_audit_plan.md`
  - source-runtime test-isolation audit notes and follow-up context

## Default workflow

1. Run request preflight.
   - Windows PowerShell:
     `powershell -ExecutionPolicy Bypass -File scripts/agent.ps1 request`
   - macOS/Linux/WSL:
     `bash scripts/agent.sh request`
2. Use `docs/TEST.md` for the right validation lane.
3. Use `docs/TROUBLESHOOTING.md` when a guardrail or environment check fails.
4. Keep changes small, update the canonical doc that owns the changed behavior,
   and avoid creating one-off docs unless the information truly needs to live
   separately.

## Principles for this folder

- Prefer a few strong documents over many narrow notes.
- Prefer current contracts over historical execution diaries.
- Keep active planning and task hierarchy in Linear, not Markdown plan files.
- Use `tmp/` only for narrow investigations or temporary implementation notes.
- Delete stale docs instead of keeping them as archaeological layers.
