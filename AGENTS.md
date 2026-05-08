# Repo Workflow

This file stays repo-specific and lightweight. Built-in Codex memory is the
durable session-memory layer; this file should only capture stable workflow and
validation expectations for `C:\dev\sempal`.

## Orientation
- Repository: `C:\dev\sempal`
- Product: Sempal
- Branch: `main`
- Linear team: `PORTALSURFER`
- Linear project: `Sempal` — https://linear.app/boostnlvp/project/sempal-7230ebfad82d
- Primary docs entrypoint: `docs/README.md`

## Planning System
- Linear is the source of truth for planning and backlog state in this repo.
- When a plan is needed, create or update Linear issues in the `Sempal` project under the `PORTALSURFER` team.
- Each planning issue must be implementation-ready in isolation:
  - clear problem statement
  - concrete scope and non-goals
  - explicit constraints and risks
  - validation steps
  - a clear definition of done
- Encode execution order in Linear with `blockedBy` / `blocks`. Use parent-child hierarchy only when it improves navigation.
- Do not use Markdown plan files such as `tmp/*.md` or `docs/plans/*` as the active plan or backlog source of truth.
- If a codebase does not yet have a Linear project, create one in `PORTALSURFER` using the codebase or crate name.

## Quick Start
1. Run repo preflight:
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/agent.ps1 request`
   - macOS/Linux/WSL: `bash scripts/agent.sh request`
2. Read the relevant repo docs for the current task:
   - `docs/README.md`
   - `docs/TEST.md`
   - `AGENTS.md`
3. If the environment looks broken:
   - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
   - macOS/Linux/WSL: `bash scripts/doctor.sh`

## Non-Negotiable Workflow Rules
- Use `main` as the default base branch for `C:\dev\sempal`; feature work should happen on a feature branch and merge through a PR.
- Keep local `main` tracking `origin/main`; the repo hook installer and `scripts/check.* main-branch` branch guard enforce the base-branch contract while allowing feature branches for PR work.
- `C:\dev\sempal\vendor\radiant` also uses local `main` tracking `origin/main`; update the submodule pointer from a Sempal feature branch and merge it through a Sempal PR.
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
- Run full CI in the platform wrapper before pushing broader validation/tooling/perf/dependency changes or when you need full CI parity (`scripts/ci.ps1 local` on Windows, `scripts/ci.sh local` elsewhere)

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
