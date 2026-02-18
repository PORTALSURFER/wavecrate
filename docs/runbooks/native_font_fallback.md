# Runbook: Native shell font fallback failures

## Symptoms

- Text rendering fails or looks incorrect.
- Logs mention font loading/discovery issues.

## Why this happens

The native shell uses a small set of system font candidates. If system font
discovery fails on a platform or environment, you can supply an explicit font
path.

## Fix

- Set `SEMPAL_NATIVE_FONT_PATH` to a known-good `.ttf` file.
- Re-run the app.

## Safe reproduction

- Run in an isolated sandbox (avoids touching real user data):
  - `bash scripts/run_sandbox.sh --`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_sandbox.ps1`

## Notes

- `scripts/doctor.*` checks whether `SEMPAL_NATIVE_FONT_PATH` points to an existing file.
- See `docs/ENV_VARS.md` for details.

