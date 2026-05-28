# Repo Workflow

This file stays repo-specific and lightweight. Built-in Codex memory is the
durable session-memory layer; this file should only capture stable workflow and
validation expectations for `C:\dev\wavecrate`.

## Orientation
- Repository: `C:\dev\wavecrate`
- Product: Wavecrate
- Branch: `main`
- Linear team: `PORTALSURFER`
- Linear project: `Wavecrate` — https://linear.app/boostnlvp/project/wavecrate-7230ebfad82d
- Primary docs entrypoint: `docs/README.md`

## Planning System
- Linear is the source of truth for planning and backlog state in this repo.
- When a plan is needed, create or update Linear issues in the `Wavecrate` project under the `PORTALSURFER` team.
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
- Use `next` as the default integration branch for `C:\dev\wavecrate`; feature work should happen on a feature branch and merge into `next`.
- Keep local `next` tracking `origin/next`; the repo hook installer and `scripts/check.* integration-branch` branch guard enforce the integration-branch contract while allowing feature branches for PR work.
- Keep `main` as the release branch. When `next` is merged into `main`, bump the Wavecrate version by one patch number in the same release merge.
- `C:\dev\wavecrate\vendor\radiant` also uses local `main` tracking `origin/main`; update the submodule pointer from a Wavecrate feature branch and merge it through a Wavecrate PR.
- During the tight edit loop, prioritize implementation speed and direct manual
  checks for the behavior under active development. Do not run formatter or CI
  after every small edit unless the edit is risky, the user asks for it, or a
  failing command is needed to understand the bug.
- Intermediate commits and pushes are allowed without running the validation
  lanes. Use them to preserve progress on feature branches; clearly report when
  a pushed branch has not yet passed the final gate.
- Normal commits and pushes to `next` do not require the agent CI lane. Use
  focused checks or smoke checks when useful for the active change, and clearly
  report any checks that were skipped.
- Before merging a PR into `next`, run formatting if code changed and run the
  final validation gate. In constrained agent-side work, the final gate is:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 agent`
  - macOS/Linux/WSL: `bash scripts/ci.sh agent`
- For broader integrated local validation built around `cargo nextest`:
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 quick`
  - macOS/Linux/WSL: `bash scripts/ci.sh quick`
- If a final validation lane fails: fix and rerun until green before merging.
- Do not run multiple Rust test commands concurrently. Keep `cargo test` / `cargo nextest` invocations to one process at a time to avoid cargo lock contention and misleading timeouts, but allow the normal in-process Rust test threading within that single test run.
- On Windows, do not run the Bash workflow scripts. Use only the PowerShell wrappers (`scripts/*.ps1`) for preflight/CI/devcheck unless the user explicitly overrides this.
- After code changes: commit and push as useful for collaboration. The final PR
  merge still requires the final validation gate to be green.
- In constrained agent environments, do not merge unless `ci_agent` is green;
  report whether `ci_quick` or `ci_local` still need a user-run confirmation pass.
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
