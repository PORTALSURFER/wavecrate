# Plans Index

`docs/plans/` is the stable docs-side landing area for plan navigation.
Use it to find the current queue, reuse the shared templates, and reach the
active `tmp/` artifacts without hard-coding transient paths into wake-up docs
or guardrails.

## Current Map

- `docs/plans/active/todo.md`
  - short ordered queue for the current lane
- `docs/plans/TEMPLATE_execution_plan.md`
  - reusable template for execution plans
- `docs/plans/TEMPLATE_investigation.md`
  - reusable template for investigation writeups
- `tmp/improvement_audit_plan.md`
  - active repo-wide improvement backlog and execution record
- `tmp/database_system_audit_plan.md`
  - database-system audit notes and follow-up context
- `tmp/source_runtime_test_isolation_audit_plan.md`
  - source-runtime test-isolation audit notes and follow-up context

## Usage Rules

- Keep `docs/plans/` small and durable.
- Put long-lived navigation and reusable templates here.
- Put active, narrow, or fast-moving execution detail in `tmp/`.
- When a `tmp/` plan becomes a repeated workflow, add or update the matching
  template here instead of growing `AGENTS.md`.
