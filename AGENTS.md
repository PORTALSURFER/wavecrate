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
   - macOS/Linux/WSL: `bash scripts/run_agent_request.sh`
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`
2. Read in order:
   - `docs/README.md`
   - `docs/plans/index.md`
   - `docs/plans/active/runtime_performance_exec_plan.md`
   - `docs/plans/active/todo.md`
   - `MEMORY.md`
3. If environment issues are suspected:
   - macOS/Linux/WSL: `bash scripts/doctor.sh`
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`

## Current Mission
- Repository: `C:\dev\sempal`
- Product: Sempal
- Branch: `next`
- Program: post-cleanup architecture audit refresh
- Source of truth: `tmp/cleanup_plan.md`
- Current status: the refreshed cleanup backlog is now in Phase 2 execution; items 1-9 are complete, 7 items remain, and item 10 is next.

## Immediate Next Actions
1. Continue cleanup strictly in `tmp/cleanup_plan.md` order at item 10.
2. After each cleanup item, rerun validation, update `tmp/cleanup_plan.md`, and commit/push.
3. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `tmp/cleanup_plan.md` synchronized after each cleanup milestone.
4. Keep `tmp/perf_plan.md` and `docs/plans/active/runtime_performance_exec_plan.md` dormant unless a separate perf lane is explicitly reopened.

## Handoff Anchors
- `MEMORY.md`: live, present-tense snapshot of what is happening now
- `docs/plans/active/todo.md`: short ordered queue for immediate actions
- `docs/plans/index.md`: active/completed plan map
- `tmp/cleanup_plan.md`: active strict ROI-ranked cleanup backlog (Phase 2 active; items 1-9 complete on 2026-03-11, item 10 next)
- `docs/plans/active/cleanup_architecture_note.md`: cleanup boundary/ownership guidance
- `tmp/perf_plan.md`: current ROI-ranked runtime performance audit backlog and execution order

## Non-Negotiable Workflow Rules
- During the tight edit loop:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - macOS/Linux/WSL: `bash scripts/devcheck.sh`
- Before commit/push and after non-trivial edits:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - macOS/Linux/WSL: `bash scripts/ci_quick.sh`
- If devcheck or quick CI fails: fix and rerun until green
- Do not run Rust test commands in multiple concurrent processes; run them serially in one process to avoid cargo lock contention and misleading timeouts
- On Windows, do not run the Bash workflow scripts. Use only the PowerShell wrappers (`scripts/*.ps1`) for preflight/CI/devcheck unless the user explicitly overrides this.
- After code changes: commit and push
- Do not push unless quick CI is green in the current platform wrapper (`ci_quick.ps1` on Windows, `ci_quick.sh` elsewhere)
- Run full CI in the platform wrapper before pushing broader validation/tooling/perf/dependency changes or when you need full CI parity (`ci_local.ps1` on Windows, `ci_local.sh` elsewhere)

## Golden Commands
- Bootstrap:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
  - macOS/Linux/WSL: `bash scripts/bootstrap.sh`
- Smoke devcheck:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - macOS/Linux/WSL: `bash scripts/devcheck.sh`
- Fast dev checks:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - macOS/Linux/WSL: `bash scripts/ci_quick.sh`
- CI parity:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - macOS/Linux/WSL: `bash scripts/ci_local.sh`
- Safe run:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1 --`
  - macOS/Linux/WSL: `bash scripts/run_sandbox.sh --`
- Clean sandbox:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/clean_sandbox.ps1`
  - macOS/Linux/WSL: `bash scripts/clean_sandbox.sh`
- Diagnostics:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
  - macOS/Linux/WSL: `bash scripts/doctor.sh`
- Latest log:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
  - macOS/Linux/WSL: `bash scripts/latest_log.sh`
- Bug bundle:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`
  - macOS/Linux/WSL: `bash scripts/bug_bundle.sh`
