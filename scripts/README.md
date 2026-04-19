# Scripts

Top-level `scripts/` now keeps only the entrypoints people are expected to run
regularly.

Primary workflow:

- `bootstrap.{sh,ps1}`
- `doctor.{sh,ps1}`
- `devcheck.{sh,ps1}`
- `ci_agent.{sh,ps1}`
- `ci_quick.{sh,ps1}`
- `ci_local.{sh,ps1}`
- `run_sandbox.{sh,ps1}`
- `clean_sandbox.{sh,ps1}`
- `latest_log.{sh,ps1}`
- `bug_bundle.{sh,ps1}`
- `audit_cleanup_hotspots.{sh,ps1}`

Grouped specialist entrypoints:

- `agent.{sh,ps1}` for request/preflight/checks
- `check.{sh,ps1}` for guardrails and report helpers
- `perf.{sh,ps1}` for runtime-performance lanes
- `gui.ps1` for GUI validation lanes

Implementation details now live in subdirectories such as `scripts/agent/`,
`scripts/check/`, `scripts/gui/`, `scripts/perf/`, `scripts/internal/`, and
`scripts/release/`.
