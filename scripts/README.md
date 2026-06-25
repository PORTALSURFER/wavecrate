# Scripts

Top-level `scripts/` is intentionally small. `scripts/command-inventory.json`
is the checked inventory for public entrypoints, compatibility wrappers, and
dispatcher maps. These are the public entrypoints people should run directly:

- `bootstrap.{sh,ps1}`: set up the repo and install hooks.
- repo-root `run.{sh,ps1}`: compatibility wrappers for `cargo run`.
- `run.ps1 logs debug-overlays` or `run.ps1 logs debug-layout`: convenience
  aliases for `cargo run -- --log` with debug layout overlays shown. The
  repo-root `.\run.ps1` delegates these public run commands to `scripts/run.ps1`.
- `doctor.{sh,ps1}`: diagnose environment issues.
- `agent.{sh,ps1}`: agent request, preflight, checks, and hook install helpers.
- `ci.{sh,ps1}`: validation lanes (`smoke`, `agent`, `quick`, `local`).
  `local` is the broad local validation lane; perf and GUI checks use their own
  entrypoints.
- `check.{sh,ps1}`: focused guardrails and report helpers.
- `run.{sh,ps1}`: sandbox, cleanup, log, and bug-bundle helpers.
- `perf.{sh,ps1}`: performance guard and calibration commands. The startup
  threshold calibration helper is currently bash-only.
- `gui.ps1`: Windows GUI validation lanes.

PowerShell compatibility wrappers also remain available for the older
single-purpose entrypoints:

- `devcheck.ps1`
- `ci_agent.ps1`
- `ci_quick.ps1`
- `ci_local.ps1`
- `run_sandbox.ps1`
- `clean_sandbox.ps1`
- `latest_log.ps1`
- `bug_bundle.ps1`

Those wrappers delegate to `ci.ps1` and `run.ps1` so existing local muscle
memory and external instructions still resolve during the migration. Keep them
until the inventory marks a wrapper as retired and every canonical repo
reference has been moved to the dispatcher form.

Everything else under `scripts/internal/` is implementation detail. Generated
training dataset artifacts should stay under ignored workspace paths such as
`tmp/training_dataset/`.
