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
   - `tmp/improvement_audit_plan.md`
   - `docs/plans/active/todo.md`
   - `MEMORY.md`
3. If environment issues are suspected:
   - macOS/Linux/WSL: `bash scripts/doctor.sh`
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`

## Current Mission
- Repository: `X:\sempal`
- Product: Sempal
- Branch: `next`
- Program: evidence-driven improvement audit execution for the current live tree
- Source of truth: `tmp/improvement_audit_plan.md` for the refreshed 2026-04-01 ROI-ranked backlog and execution record for this tree; `docs/TEST.md` and `docs/README.md` still define the validation workflow; `docs/gui_test_platform.md`, `tmp/cleanup_plan.md`, and `tmp/perf_plan.md` remain relevant background references
- Current status: Phase 2 is active on `2026-04-01`. Items 1, 2, and 3 are complete, item 4 (`app_core` dispatch-hub splitting) is next, and execution is proceeding in backlog order.

## Immediate Next Actions
1. Execute item 4 from `tmp/improvement_audit_plan.md`: finish the `app_core` dispatch-hub split so migration-facing routing depends on narrower controller seams.
2. Continue the backlog strictly in order after item 4, with the remaining production/runtime file-size debt next.
3. Keep `tmp/cleanup_plan.md` and `tmp/perf_plan.md` parked unless the user explicitly reopens those lanes.
4. Keep the PowerShell validation wrappers on their direct-`rustc`/repo-temp fallback path whenever inherited `sccache` or the default temp dir is unusable in this environment.
5. Keep `AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`, and `docs/plans/index.md` synchronized when the lane status changes.

## Handoff Anchors
- `MEMORY.md`: live, present-tense snapshot of what is happening now
- `docs/plans/active/todo.md`: short ordered queue for immediate actions
- `docs/plans/index.md`: active/completed plan map
- `tmp/improvement_audit_plan.md`: refreshed evidence-driven ROI-ranked improvement backlog and execution record for the live tree; Phase 2 is active, items 1, 2, and 3 are complete, and item 4 is next
- `docs/gui_test_platform.md`: GUI action catalog, automation snapshot, test mode, CLI, and AIV architecture
- `docs/plans/active/gui_test_platform_exec_plan.md`: phased implementation plan for the GUI automation/test platform
- `tmp/cleanup_plan.md`: parked strict ROI-ranked cleanup backlog rebuilt on `2026-03-12`; resume only after explicit cleanup confirmation
- `docs/plans/active/cleanup_architecture_note.md`: cleanup boundary/ownership guidance
- `tmp/perf_plan.md`: current ROI-ranked runtime performance audit backlog and execution order

## Non-Negotiable Workflow Rules
- Use `next` as the development branch for both `C:\dev\sempal` and `C:\dev\sempal\vendor\radiant` unless the user explicitly directs otherwise.
- Keep both repos on local `next` tracking `origin/next`; the repo hook installer and `scripts/check_next_branch.*` are the enforcement path.
- During the tight edit loop:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - macOS/Linux/WSL: `bash scripts/devcheck.sh`
- For constrained agent-side validation before commit/push and after non-trivial edits:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - macOS/Linux/WSL: `bash scripts/ci_agent.sh`
- For broader integrated local validation built around `cargo nextest`:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
  - macOS/Linux/WSL: `bash scripts/ci_quick.sh`
- If devcheck or the active validation lane fails: fix and rerun until green
- Do not run Rust test commands in multiple concurrent processes; run them serially in one process to avoid cargo lock contention and misleading timeouts
- On Windows, do not run the Bash workflow scripts. Use only the PowerShell wrappers (`scripts/*.ps1`) for preflight/CI/devcheck unless the user explicitly overrides this.
- After code changes: commit and push
- In constrained agent environments, do not push unless `ci_agent` is green; report whether `ci_quick` or `ci_local` still need a user-run confirmation pass
- Run full CI in the platform wrapper before pushing broader validation/tooling/perf/dependency changes or when you need full CI parity (`ci_local.ps1` on Windows, `ci_local.sh` elsewhere)

## Golden Commands
- Bootstrap:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
  - macOS/Linux/WSL: `bash scripts/bootstrap.sh`
- Smoke devcheck:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
  - macOS/Linux/WSL: `bash scripts/devcheck.sh`
- Agent-safe validation:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`
  - macOS/Linux/WSL: `bash scripts/ci_agent.sh`
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
