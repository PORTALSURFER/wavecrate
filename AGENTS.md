# Agent Wake-Up Portal

Purpose: this file is a minimal orientation map for stateless agents.
Do not store deep specs or long plans here. Put details in `docs/` and link to
those docs.

## Persistent Context Awareness
You operate in a stateless environment and do not retain working memory
between sessions. Without a clearly defined path in `AGENTS.md`, you will lose
track of objectives, progress, and intent.

This section must permanently remain inside `AGENTS.md`.
It ensures that every time you wake up, you remember how to orient yourself.

`AGENTS.md` is your core memory file.
It is loaded whenever you wake up and serves as your reliable bridge to prior
sessions.

`AGENTS.md` must remain minimal.
It is not a knowledge base; it is a portal.

Its purpose is to:
- provide immediate orientation on wake-up
- define current goals
- link to authoritative, larger documents
- point to instrumentation and workflow systems

Large explanations, deep specifications, architectural breakdowns, and detailed
plans must live in dedicated documents (for example, `docs/*.md`).
`AGENTS.md` should only reference them with short descriptions and clear paths.

If `AGENTS.md` becomes too large, it will consume working memory at wake-up
and obscure critical context.

When writing or updating it:
- assume your next self knows nothing
- make the path back to purpose explicit
- clearly state what you were doing and why
- ensure important documents are easy to find
- remove ambiguity and outdated references

Write for future selves: be precise, kind, and clear.

## 60-Second Wake-Up
1. Run preflight:
   - macOS/Linux/WSL: `bash scripts/agent.sh request`
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/agent.ps1 request`
2. Read in order:
   - `docs/README.md`
   - `docs/plans/index.md`
   - `docs/plans/active/todo.md`
   - `tmp/improvement_audit_plan.md`
   - `MEMORY.md`
3. If environment issues are suspected:
   - macOS/Linux/WSL: `bash scripts/doctor.sh`
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`

## Current Mission
- Repository: `X:\sempal`
- Product: Sempal
- Branch: `next`
- Linear project: `Sempal` in team `PORTALSURFER` — https://linear.app/boostnlvp/project/sempal-7230ebfad82d
- Program: Improvement-audit backlog execution and documentation/guardrail upkeep for the current live tree
- Source of truth: `tmp/improvement_audit_plan.md` for the active repo-wide improvement backlog; `docs/TEST.md` and `docs/README.md` define the validation workflow; `docs/plans/index.md` and `docs/plans/active/todo.md` provide the docs-side navigation layer
- Current status: The improvement-audit lane is active in the live tree, and the current requested work should extend from the documented backlog and current user direction without reviving removed docs or stale plan paths.

## Immediate Next Actions
1. Treat `tmp/improvement_audit_plan.md` as the active backlog source of truth unless the user explicitly redirects to another lane.
2. Use `docs/plans/index.md` and `docs/plans/active/todo.md` as the stable docs-side orientation layer for current work.
3. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable in this environment.

## Handoff Anchors
- `MEMORY.md`: live, present-tense snapshot of what is happening now
- `docs/plans/active/todo.md`: short ordered queue for immediate actions
- `docs/plans/index.md`: active/completed plan map
- `tmp/improvement_audit_plan.md`: active evidence-driven repo-wide improvement backlog and execution record
- `tmp/database_system_audit_plan.md`: database-system audit notes and follow-up context
- `tmp/source_runtime_test_isolation_audit_plan.md`: source-runtime test-isolation audit notes and follow-up context
- `docs/SYSTEMS.md`: GUI automation/test platform, runtime contracts, recovery rules, and data-format notes

## Non-Negotiable Workflow Rules
- Use `next` as the development branch for both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- Keep both repos on local `next` tracking `origin/next`; the repo hook installer and `scripts/check.* next-branch` are the enforcement path.
- During the tight edit loop:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 smoke`
  - macOS/Linux/WSL: `bash scripts/ci.sh smoke`
- For constrained agent-side validation before commit/push and after non-trivial edits:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
  - macOS/Linux/WSL: `bash scripts/ci.sh agent`
- For broader integrated local validation built around `cargo nextest`:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 quick`
  - macOS/Linux/WSL: `bash scripts/ci.sh quick`
- If devcheck or the active validation lane fails: fix and rerun until green
- Do not run multiple Rust test commands concurrently. Keep `cargo test` / `cargo nextest` invocations to one process at a time to avoid cargo lock contention and misleading timeouts, but allow the normal in-process Rust test threading within that single test run.
- On Windows, do not run the Bash workflow scripts. Use only the PowerShell wrappers (`scripts/*.ps1`) for preflight/CI/devcheck unless the user explicitly overrides this.
- After code changes: commit and push
- In constrained agent environments, do not push unless `ci_agent` is green; report whether `ci_quick` or `ci_local` still need a user-run confirmation pass
- Run full CI in the platform wrapper before pushing broader validation/tooling/perf/dependency changes or when you need full CI parity (`ci_local.ps1` on Windows, `ci_local.sh` elsewhere)

## Golden Commands
- Bootstrap:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
  - macOS/Linux/WSL: `bash scripts/bootstrap.sh`
- Smoke devcheck:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 smoke`
  - macOS/Linux/WSL: `bash scripts/ci.sh smoke`
- Agent-safe validation:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
  - macOS/Linux/WSL: `bash scripts/ci.sh agent`
- Fast dev checks:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 quick`
  - macOS/Linux/WSL: `bash scripts/ci.sh quick`
- CI parity:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 local`
  - macOS/Linux/WSL: `bash scripts/ci.sh local`
- Safe run:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 sandbox --`
  - macOS/Linux/WSL: `bash scripts/run.sh sandbox --`
- Clean sandbox:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 clean`
  - macOS/Linux/WSL: `bash scripts/run.sh clean`
- Diagnostics:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
  - macOS/Linux/WSL: `bash scripts/doctor.sh`
- Latest log:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 logs`
  - macOS/Linux/WSL: `bash scripts/run.sh logs`
- Bug bundle:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run.ps1 bug-bundle`
  - macOS/Linux/WSL: `bash scripts/run.sh bug-bundle`
