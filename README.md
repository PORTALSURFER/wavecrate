
⚠️ **Warning:** Early alpha software. Use at your own risk, this tool can modify, rename, or delete files, and bugs could damage your sample library. Keep backups and proceed with caution. ⚠️

# WAVECRATE

Audio sample triage tool built with Rust.

https://portalsurfer.org/wavecrate/  
https://portalsurfer.org/wavecrate/docs

## Platform support

Wavecrate app builds currently support macOS and Windows. Linux is not
currently supported for app installs.

Linux, WSL, and headless ALSA references in repository scripts and developer
docs are for CI, agent, and contributor validation only; they do not describe a
shipped Linux product platform.

## Storage and logs

Use **Options -> Open config folder** in the app when you need logs, settings,
or support context.

- macOS: `~/Library/Application Support/.wavecrate/`
- Windows: `%APPDATA%\\.wavecrate\\`

Logs live in the `logs/` subfolder of the active profile. Wavecrate writes
`wavecrate_<timestamp>_<run>_<sequence>.log` segments on a background logging
worker. It normally keeps at most ten matching regular log files, each up to
10 MiB. One indivisible event larger than 10 MiB stays whole in its own segment,
so that segment can exceed 10 MiB by the event's excess. Existing oversized logs
are not rewritten; they age out through ordinary oldest-first cleanup. If
rotation or cleanup fails, Wavecrate reports degraded retention and keeps the
current writable log when possible, so these limits can be exceeded until the
failure clears.

For troubleshooting, run `scripts/run.sh logs` on macOS or `scripts/run.ps1 logs`
on Windows to inspect the newest segment. `bug-bundle` collects the newest
regular Wavecrate log segments and config for support; review the archive before
sharing because logs and config can contain local paths.
