
⚠️ **Warning:** Early alpha software. Use at your own risk, this tool can modify, rename, or delete files, and bugs could damage your sample library. Keep backups and proceed with caution. ⚠️

# Yield Legacy

Archived pre-GPUI implementation of the Yield sample manager. The source and
its stored-data format are preserved as historical reference; active development
continues in [PORTALSURFER/yield](https://github.com/PORTALSURFER/yield).

## Platform support

Archived app builds supported macOS and Windows. Linux was not supported for
app installs.

Linux, WSL, and headless ALSA references in repository scripts and developer
docs are for CI, agent, and contributor validation only; they do not describe a
shipped Linux product platform.

## Storage and logs

Use **Options -> Open config folder** in the app when you need logs, settings,
or support context.

Launch logs rotate before a complete event would cross 10 MiB and successful
cleanup retains at most ten matching regular log files per profile. A single
event larger than 10 MiB stays whole and may exceed that size; rotation or
cleanup failures are reported and may temporarily exceed these bounds.

- macOS: `~/Library/Application Support/.wavecrate/`
- Windows: `%APPDATA%\\.wavecrate\\`
