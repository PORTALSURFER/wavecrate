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
1. Run preflight: `bash scripts/run_agent_request.sh`
2. Read in order:
   - `docs/README.md`
   - `docs/plans/index.md`
   - `docs/plans/active/runtime_performance_exec_plan.md`
   - `docs/plans/active/todo.md`
   - `MEMORY.md`
3. If environment issues are suspected: `bash scripts/doctor.sh`

## Current Mission
- Repository: `/home/portalsurfer/dev/sempal`
- Product: Sempal
- Branch: `next`
- Program: runtime responsiveness/performance redesign (Xilem-inspired scoped invalidation + cache reuse)
- Source of truth: `docs/plans/active/runtime_performance_exec_plan.md`
- Current status: the `tmp/perf_plan.md` runtime performance execution backlog is complete through item 11.

## Immediate Next Actions
1. Use `docs/plans/active/runtime_performance_exec_plan.md` for any follow-up perf work beyond the completed `tmp/perf_plan.md` backlog.
2. Keep handoff docs synchronized on future perf milestone commits (`AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`).
3. Execute `tmp/cleanup_plan.md` Phase 2 strictly in order, with item 5 complete and item 6 next.
4. Keep `tmp/perf_plan.md` as the completed execution record for items 1-11.

## Handoff Anchors
- `MEMORY.md`: live, present-tense snapshot of what is happening now
- `docs/plans/active/todo.md`: short ordered queue for immediate actions
- `docs/plans/index.md`: active/completed plan map
- `tmp/cleanup_plan.md`: ROI-ranked cleanup backlog + execution checklist (last refresh: 2026-03-09, 7 pending items; item 6 next)
- `docs/plans/active/cleanup_architecture_note.md`: cleanup boundary/ownership guidance
- `tmp/perf_plan.md`: current ROI-ranked runtime performance audit backlog and execution order

## Non-Negotiable Workflow Rules
- During the tight edit loop: `bash scripts/devcheck.sh`
- Before commit/push and after non-trivial edits: `bash scripts/ci_quick.sh`
- If devcheck or quick CI fails: fix and rerun until green
- After code changes: commit and push
- Do not push unless `scripts/ci_quick.sh` is green
- Run `bash scripts/ci_local.sh` before pushing broader validation/tooling/perf/dependency changes or when you need full CI parity

## Golden Commands
- Bootstrap: `bash scripts/bootstrap.sh`
- Smoke devcheck: `bash scripts/devcheck.sh`
- Fast dev checks: `bash scripts/ci_quick.sh`
- CI parity: `bash scripts/ci_local.sh`
- Safe run: `bash scripts/run_sandbox.sh --`
- Clean sandbox: `bash scripts/clean_sandbox.sh`
- Diagnostics: `bash scripts/doctor.sh`
- Latest log: `bash scripts/latest_log.sh`
- Bug bundle: `bash scripts/bug_bundle.sh`
