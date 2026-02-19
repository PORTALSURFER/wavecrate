# Runbook: Updater path validation failures (symlink checks)

## Symptoms

- Updates fail with errors about refusing to write through symlinks.
- Updates fail with “Failed to validate update path for symlinks …”.

## Why this happens

The updater validates that install paths do not traverse symlinks to avoid
writing outside the intended install directory.

## Fix

- Preferred: install/update in a directory without symlink components.
- If failures are due to permission or transient I/O errors during validation:
  - Set `SEMPAL_UPDATER_ALLOW_SYMLINK_ERRORS=1` for dev/test only.

## Diagnostics

- Tail newest log:
  - `bash scripts/latest_log.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`

## Notes

- See `docs/ENV_VARS.md` for the override semantics.

## Trust and AV diagnostics (Windows)

- Windows update install is manual by design. The app opens the release page and
  does not replace binaries in place.
- Release CI signs `sempal.exe`, `sempal-updater.exe`, and
  `sempal-installer.exe` using Authenticode and fails the build if signature
  verification is not `Valid`.
- If endpoint protection still flags a release:
  - Confirm the affected executable reports `Valid` with
    `Get-AuthenticodeSignature`.
  - Record the SHA-256 of the flagged file and submit a false-positive report to
    your AV vendor with the signature details.
