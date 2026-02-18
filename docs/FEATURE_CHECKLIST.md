# How to Add a Feature Safely

This checklist is the default “safe path” for implementing changes in Sempal.
It is written to be scannable for both humans and coding agents.

## 1) Decide where the code should live

- Read `docs/design_principles.md` for architectural constraints and conventions.
- Prefer adding new domain logic under `src/` (not in `vendor/`).
- If the change is UI behavior/layout/input routing, prefer implementing it in `vendor/radiant` and keep `src` limited to intent + domain state.
- If the change is app projection/intent wiring, prefer `src/app_core` over `src/app` (legacy) unless you are explicitly working in the legacy runtime.
- If you are unsure, start by identifying which module “owns” the behavior using the module map in `AGENTS.md`.

## 2) Define the behavior you are adding

- Write down the user-visible behavior, including edge cases and failure modes.
- Identify constraints: performance expectations, determinism, data migration, and platform support (Windows/macOS/Linux).
- Decide what should be logged, and at what boundary.

## 3) Add tests for non-trivial logic

- Use `docs/TEST.md` to find the right test suite and the preferred command.
- Favor fast unit tests near the code; use integration tests when behavior crosses module boundaries.
- Cover:
- “happy path”
- boundary cases (empty inputs, large datasets, odd filenames, long paths)
- failure paths (I/O failures, malformed data, missing optional dependencies)

## 4) Update documentation close to the change

- Update user-facing docs when behavior changes:
- `manual/usage.md` for workflows and UI-facing behavior
- `README.md` for build/run/env-var guidance
- Update developer docs in `docs/` when behavior changes (architecture, invariants, env vars, tests).
- Update or add module-level docs (`//!`) when responsibilities or invariants change.

## 5) Run the “golden path” checks locally

- On macOS/Linux/WSL: `bash scripts/ci_local.sh`
- On Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- If you hit environment issues, run:
- `bash scripts/doctor.sh` or `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`

## 6) Logging and diagnostics

- Logs are intended to capture intent and outcomes, not internal noise.
- Avoid logging sensitive or personal data (paths may be sensitive in some contexts).
- Where to find logs:
- Linux: `$HOME/.config/.sempal/logs`
- macOS: `$HOME/Library/Application Support/.sempal/logs`
- Windows: `%APPDATA%\\.sempal\\logs`

## 7) Keep diffs easy to review

- Prefer small, focused commits and minimal surface area.
- Refactor large files into focused modules when changes would deepen nesting or expand already-large units.
- If you touch `vendor/radiant`, keep the contract boundaries described in `README.md` intact.
