# Runbook: Windows ASIO build failures

## Symptoms

- Build fails on Windows when compiling `cpal` with ASIO support.
- Errors mention missing ASIO headers (for example `asiodrivers.h`).

## Why this happens

The Windows build enables the `cpal` `asio` feature and requires the Steinberg
ASIO SDK headers. The SDK path is provided via `CPAL_ASIO_DIR`.

## Fix

- Confirm you have the ASIO SDK available locally.
- Set `CPAL_ASIO_DIR` to the ASIO SDK root directory that contains `host/` and `common/`.
- Re-run the build.

## Quick checks

- Validate environment: `powershell -ExecutionPolicy Bypass -File scripts/doctor.ps1`
- Confirm the folder layout:
  - `<CPAL_ASIO_DIR>/host`
  - `<CPAL_ASIO_DIR>/common`

## Notes

- CI fetches the ASIO SDK automatically; local builds must set `CPAL_ASIO_DIR`.
- See `docs/ENV_VARS.md` for details.

