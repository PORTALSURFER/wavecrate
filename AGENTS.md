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
- Current status: `tmp/perf_plan.md` Phase 2 execution is in progress; items 1-6 are complete (browser focus hot path, waveform static/motion key partitioning, deferred transients, stale-aware reads, retained-model miss-path cleanup, motion status projection trimming), and item 7 is next.

## Immediate Next Actions
1. Await user confirmation to begin Phase 2 implementation from `tmp/cleanup_plan.md` (refreshed 2026-03-04).
2. Execute item 7 from `tmp/perf_plan.md`: cache waveform image upload payloads across draws.
3. Continue active plan items sequentially in strict ROI order; after each item run validation, commit, push, and mark completion with date/hash.
4. Keep handoff docs synchronized on milestone commits (`AGENTS.md`, `MEMORY.md`, `docs/plans/active/todo.md`).

## Handoff Anchors
- `MEMORY.md`: live, present-tense snapshot of what is happening now
- `docs/plans/active/todo.md`: short ordered queue for immediate actions
- `docs/plans/index.md`: active/completed plan map
- `tmp/cleanup_plan.md`: ROI-ranked cleanup backlog + execution checklist (last refresh: 2026-03-04)
- `tmp/perf_plan.md`: ROI-ranked runtime performance backlog + execution checklist for the current perf pass

## Non-Negotiable Workflow Rules
- Before and after edits: `bash scripts/ci_local.sh`
- If CI fails: fix and rerun until green
- After code changes: commit and push
- Do not push unless `scripts/ci_local.sh` is green

## Golden Commands
- Bootstrap: `bash scripts/bootstrap.sh`
- CI parity: `bash scripts/ci_local.sh`
- Safe run: `bash scripts/run_sandbox.sh --`
- Clean sandbox: `bash scripts/clean_sandbox.sh`
- Diagnostics: `bash scripts/doctor.sh`
- Latest log: `bash scripts/latest_log.sh`
- Bug bundle: `bash scripts/bug_bundle.sh`
