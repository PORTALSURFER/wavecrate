# Developer documentation

The `docs/` directory contains developer-facing documentation for Sempal.

User-facing documentation lives in `manual/` (usage guide + the published docs
site).

Start here:

- `docs/INDEX.md` — invariants + allowlists inventory (what to do when checks fail)
- `docs/FEATURE_CHECKLIST.md` — safe path for implementing changes
- `docs/ARCHITECTURE.md` — module ownership map
- `docs/ENV_VARS.md` — environment variable reference
- `docs/TEST.md` — test suite map and commands
- `docs/design_principles.md` — architectural goals and constraints
- `docs/QUALITY_SCORE.md` — coarse quality scorecard and known gaps
- `docs/plans/TEMPLATE_execution_plan.md` — template for multi-step work
- `docs/plans/TEMPLATE_investigation.md` — template for bug/perf investigations

## Run / diagnose / CI parity

Use these scripts as the default entrypoints for local work (humans and agents).

- Bootstrap tooling + pinned toolchain:
  - macOS/Linux/WSL: `bash scripts/bootstrap.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
- CI parity checks:
  - macOS/Linux/WSL: `bash scripts/ci_local.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Safe local run (isolated config/logs):
  - macOS/Linux/WSL: `bash scripts/run_sandbox.sh --`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1`
- Environment sanity checks:
  - macOS/Linux/WSL: `bash scripts/doctor.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
- Latest log tail:
  - macOS/Linux/WSL: `bash scripts/latest_log.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
- Bug report bundle (logs + config + versions):
  - macOS/Linux/WSL: `bash scripts/bug_bundle.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`

## Runbooks

Fast triage for common failure modes:

- `docs/runbooks/asio_build_failures.md`
- `docs/runbooks/keyring_failures.md`
- `docs/runbooks/native_font_fallback.md`
- `docs/runbooks/sqlite_extension_load_blocked.md`
- `docs/runbooks/updater_path_validation.md`
