# Runbook: Keyring failures (issue reporting token storage)

## Symptoms

- GitHub issue submission fails with messages about “Keyring unavailable”.
- Token cannot be stored or retrieved.

## Why this happens

Issue reporting prefers OS keyring storage. On some systems the keyring backend
is unavailable, misconfigured, or blocked.

## Fix options

- Preferred: fix the OS keyring backend (platform-specific).
- Workaround: disable keyring usage and opt into encrypted fallback storage:
  - Set `SEMPAL_DISABLE_KEYRING=1`
  - Set `SEMPAL_ALLOW_FALLBACK_TOKEN_STORAGE=1`
  - Provide the fallback encryption key via `SEMPAL_FALLBACK_KEY` (base64-encoded 32 bytes)

## Diagnostics

- Tail newest log:
  - `bash scripts/latest_log.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`
- Create a bundle for a bug report:
  - `bash scripts/bug_bundle.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/bug_bundle.ps1`

## Notes

- Review `docs/ENV_VARS.md` for the exact semantics of the token-store env vars.

