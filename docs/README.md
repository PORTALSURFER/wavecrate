# Developer documentation

The `docs/` directory contains developer-facing documentation for Sempal.

User-facing documentation lives in `manual/` (usage guide + the published docs
site).

Start here:

- `AGENTS.md` (repo root) — minimal wake-up portal + current mission
- `MEMORY.md` (repo root) — current session snapshot ("what is happening now")
- `docs/INDEX.md` — invariants + allowlists inventory (what to do when checks fail)
- `docs/FEATURE_CHECKLIST.md` — safe path for implementing changes
- `docs/ARCHITECTURE.md` — module ownership map
- `docs/file_ops_journal_recovery.md` — file-op journal stage contract and startup recovery rules
- `docs/folder_delete_recovery.md` — folder-delete staging and restore/finalize recovery contract
- `docs/native_bridge_projection_cache.md` — retained native-bridge segment keys, invalidation boundaries, and profiling/assertion contract
- `docs/ENV_VARS.md` — environment variable reference
- `docs/build_speed.md` — local compile-speed workflow and crate-split sketch
- `docs/TEST.md` — test suite map and commands
- `docs/design_principles.md` — architectural goals and constraints
- `docs/radiant_slot_layout_spec.md` — strict hierarchical slot-based layout contract for `vendor/radiant`
- `docs/QUALITY_SCORE.md` — coarse quality scorecard and known gaps
- `docs/plans/index.md` — current/archived plan index for parallel agents
- `docs/plans/active/todo.md` — short ordered queue for the active execution lane
- `docs/plans/TEMPLATE_execution_plan.md` — template for multi-step work
- `docs/plans/TEMPLATE_investigation.md` — template for bug/perf investigations
- `docs/run_contracts.md` — machine-readable app-run artifact contract

## Run / diagnose / CI parity

Use these scripts as the default entrypoints for local work (humans and agents).

- Bootstrap tooling + pinned toolchain:
  - macOS/Linux/WSL: `bash scripts/bootstrap.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
- Fastest smoke/compile checks:
  - Optional lighter app-only loop:
    - macOS/Linux/WSL: `bash scripts/devcheck_app.sh`
    - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck_app.ps1`
  - This intentionally skips support-tool bins and tests; still run `devcheck` before commit.
  - macOS/Linux/WSL: `bash scripts/devcheck.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/devcheck.ps1`
- Fast development checks:
  - macOS/Linux/WSL: `bash scripts/ci_quick.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_quick.ps1`
- Full local validation gate:
  - macOS/Linux/WSL: `bash scripts/ci_local.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
  - This is broader than GitHub CI because it also runs `scripts/run_perf_guard.sh`.
- Agent request preflight:
  - `bash scripts/run_agent_request.sh` (or `powershell -ExecutionPolicy Bypass -File scripts/run_agent_request.ps1`)  
    refreshes `MEMORY.md`, runs mandatory checks, then the smoke `devcheck` by default.
  - Pass `--quick-ci` for the filtered fast test loop or `--full-ci` for the broader local validation gate.
- Lightweight per-request preflight: `bash scripts/run_agent_preflight.sh`
- Automatic pull/checkout enforcement:
  `bash scripts/install_agent_preflight_hooks.sh` is installed by
  `bash scripts/bootstrap.sh` and enforces lightweight preflight checks after
  branch/source updates.
  - Configure via `AGENT_PREFLIGHT_UPDATER` and
    `AGENT_PREFLIGHT_MEMORY_MAX_AGE_HOURS`.
  - Skip hook execution with `SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK=1`.
  - Skip bootstrap-time hook installation with
    `SEMPAL_SKIP_AGENT_PREFLIGHT_HOOK_INSTALL=1`.
  - CI-level memory guardrail overrides: `AGENT_CI_REQUIRED_UPDATER` and
    `AGENT_CI_MEMORY_MAX_AGE_HOURS` (defaults: unset and `24`).
- Safe local run (sandboxed config/logs):
  - Default sandbox is persistent under `<repo>/.sandbox/sempal` for easy inspection.
  - Ephemeral sandbox (no state left behind): `bash scripts/run_sandbox.sh --temp --` / `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1 -Temp --`
  - Per-source `.sempal_samples.db` files are read-only by default (`--write-db` required to allow writes).
  - `--allow-user-library-db-write` is required to write DB files under user-library-like source roots.
  - Run:
    - macOS/Linux/WSL: `bash scripts/run_sandbox.sh --`
    - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1 --`
- Clean sandbox state (delete `<repo>/.sandbox/sempal`):
  - macOS/Linux/WSL: `bash scripts/clean_sandbox.sh`
  - Windows PowerShell: `powershell -ExecutionPolicy Bypass -File scripts/clean_sandbox.ps1`
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
