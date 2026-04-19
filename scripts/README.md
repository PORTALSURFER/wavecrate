# Scripts

Top-level `scripts/` is intentionally small. These are the public entrypoints
people should run directly:

- `bootstrap.{sh,ps1}`: set up the repo and install hooks.
- `doctor.{sh,ps1}`: diagnose environment issues.
- `agent.{sh,ps1}`: agent request, preflight, checks, and hook install helpers.
- `ci.{sh,ps1}`: validation lanes (`smoke`, `agent`, `quick`, `local`).
- `check.{sh,ps1}`: focused guardrails and report helpers.
- `run.{sh,ps1}`: sandbox, cleanup, log, and bug-bundle helpers.
- `perf.{sh,ps1}`: performance guard and calibration commands.
- `gui.ps1`: Windows GUI validation lanes.

Everything else under `scripts/internal/` is implementation detail. Tracked
training dataset artifacts that used to live under `scripts/` now live under
`testdata/training_dataset/scripts_dataset/`.
