# Troubleshooting and Guardrails

Use this document when local setup, validation, or invariant checks fail.

## First diagnostics

- environment sanity:
  - Windows PowerShell:
    `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
  - macOS/Linux/WSL:
    `bash scripts/doctor.sh`
- latest logs:
  - Windows PowerShell:
    `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
  - macOS/Linux/WSL:
    `bash scripts/latest_log.sh`
- bug bundle:
  - Windows PowerShell:
    `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`
  - macOS/Linux/WSL:
    `bash scripts/bug_bundle.sh`

## Common failures

### Windows ASIO build failures

Symptoms:

- `cpal` build errors mentioning missing ASIO headers

Fix:

- set `CPAL_ASIO_DIR` to the ASIO SDK root containing `host/` and `common/`

### Keyring failures

Symptoms:

- issue-report token storage fails because the OS keyring is unavailable

Fix:

- preferred: repair the local keyring backend
- fallback:
  - `SEMPAL_DISABLE_KEYRING=1`
  - `SEMPAL_ALLOW_FALLBACK_TOKEN_STORAGE=1`
  - `SEMPAL_FALLBACK_KEY=<base64 32-byte key>`

### Native font fallback issues

Symptoms:

- text rendering fails or the runtime cannot resolve a usable system font

Fix:

- set `SEMPAL_NATIVE_FONT_PATH` to a known-good `.ttf`

### SQLite extension load blocked

Symptoms:

- vector-acceleration extension is skipped or blocked

Fix:

- set both `SEMPAL_SQLITE_EXT` and `SEMPAL_SQLITE_EXT_ENABLE=1`
- keep the extension under the allowed path unless you are intentionally using
  the unsafe mode and matching cargo feature

### Updater path validation failures

Symptoms:

- updater refuses to write through symlinked paths

Fix:

- prefer a non-symlink install path
- for dev/test-only validation issues, use
  `SEMPAL_UPDATER_ALLOW_SYMLINK_ERRORS=1`

## Guardrail policy

Most repository invariants are enforced by `scripts/check_*.{sh,ps1}`.

When a check fails:

1. fix the underlying issue if possible
2. use an allowlist only as a last resort
3. keep the error message and owning doc in sync
4. wire invariant changes through both shell and PowerShell wrappers

## Changing an invariant safely

1. update the check implementation
2. update or add the matching allowlist file if exceptions are required
3. wire the check into the correct CI/local wrapper
4. update the canonical docs in this folder if the workflow or boundary changed
5. rerun the relevant validation lane until it is green

## When to add a new doc

Usually, do not.

Add a new long-lived doc only when the information cannot fit cleanly into one
of the canonical docs without making that file misleading. Historical plans,
audits, and temporary investigations should go to `docs/plans/` or `tmp/`.
