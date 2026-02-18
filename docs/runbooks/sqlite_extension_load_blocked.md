# Runbook: SQLite extension load blocked

## Symptoms

- Logs show SQLite extension load was skipped or blocked.
- The app behaves as if vector acceleration is disabled.

## Why this happens

SQLite extension loading is intentionally opt-in and constrained for safety:

- `SEMPAL_SQLITE_EXT` selects the extension path.
- `SEMPAL_SQLITE_EXT_ENABLE=1` is required to opt in.
- By default, the extension must live under `<app_root>/sqlite_extensions`.
- Unsafe mode requires both `SEMPAL_SQLITE_EXT_UNSAFE=1` and a build with the
  `sqlite-ext-unsafe` cargo feature.

## Fix

- Ensure you explicitly opt in:
  - `SEMPAL_SQLITE_EXT=/path/to/extension`
  - `SEMPAL_SQLITE_EXT_ENABLE=1`
- Ensure the extension path meets the safety constraints for your build.
- If you need unsafe mode:
  - Rebuild with `--features sqlite-ext-unsafe`
  - Set `SEMPAL_SQLITE_EXT_UNSAFE=1`

## Diagnostics

- Tail newest log:
  - `bash scripts/latest_log.sh`
  - `powershell -ExecutionPolicy Bypass -File scripts/latest_log.ps1`

## Notes

- See `docs/ENV_VARS.md` and `src/sqlite_ext.rs` for the exact rules.

