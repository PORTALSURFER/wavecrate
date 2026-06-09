# Scripts

Top-level `scripts/` is intentionally small. `scripts/command-inventory.json`
is the checked inventory for public entrypoints, compatibility wrappers, and
dispatcher maps. These are the public entrypoints people should run directly:

- `bootstrap.{sh,ps1}`: set up the repo and install hooks.
- `registered-run.ps1`: build a registered Wavecrate release binary,
  stage/deploy it through a local `portalsurfer.org` checkout, then launch the
  built app with forwarded args. The script finds `portalsurfer.org` beside the
  Wavecrate checkout, or you can set `WAVECRATE_PORTALSURFER_ROOT` / pass
  `-PortalSurferRoot`. Build ids use the server-side build counter format
  `wavecrate-b<N>-<timestamp>-<gitsha>`. Example:
  `powershell -ExecutionPolicy Bypass -File scripts/registered-run.ps1 -AppArgs --log`.
  For local-only testing, add `-Internal` to build without registration,
  staging, or deployment. Examples:
  `powershell -ExecutionPolicy Bypass -File scripts/registered-run.ps1 -Internal -AppArgs --log`
  and
  `powershell -ExecutionPolicy Bypass -File scripts/registered-run.ps1 -Internal -Profile debug -AppArgs --log`.
- `internal-run.ps1`: run a release-profile internal Wavecrate build with
  registration disabled and logging enabled. It runs from the repo root and does
  not need the website checkout. Example:
  `powershell -ExecutionPolicy Bypass -File scripts/internal-run.ps1`.
- `run.ps1 logs debug-overlays` or `run.ps1 logs debug-layout`: convenience
  aliases for the internal non-sandbox run path with logging enabled and debug
  layout overlays shown. The repo-root `.\run.ps1` delegates these public run
  commands to `scripts/run.ps1`.
- `doctor.{sh,ps1}`: diagnose environment issues.
- `agent.{sh,ps1}`: agent request, preflight, checks, and hook install helpers.
- `ci.{sh,ps1}`: validation lanes (`smoke`, `agent`, `quick`, `local`).
  `local` is required CI parity; perf and GUI checks use their own entrypoints.
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
