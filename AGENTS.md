
# Agent entrypoints

Read first:

- `docs/README.md` (developer docs landing page)
- `docs/FEATURE_CHECKLIST.md` (safe path for changes)
- `docs/ARCHITECTURE.md` (ownership + where changes should go)
- `docs/ENV_VARS.md` (env var reference, including ASIO build notes)
- `docs/design_principles.md` (architecture goals/constraints)

Golden path commands (CI parity + diagnostics):

- Bootstrap tooling + pinned toolchain: `bash scripts/bootstrap.sh` or `powershell -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
- CI parity checks: `bash scripts/ci_local.sh` or `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Safe local run (isolated config/logs): `bash scripts/run_sandbox.sh --` or `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1`
- Environment sanity checks: `bash scripts/doctor.sh` or `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
- Tail newest log: `bash scripts/latest_log.sh` or `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
- Create bug bundle: `bash scripts/bug_bundle.sh` or `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`

Rules:

- After any code change, create a commit and push it.
  If your environment requires explicit approval for git operations, ask for confirmation and include the intended commit message.
- `manual/` is user-facing documentation only. Developer docs belong in `docs/`.
