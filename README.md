
⚠️ **Warning:** Early alpha software. Use at your own risk, this tool can modify, rename, or delete files, and bugs could damage your sample library. Keep backups and proceed with caution. ⚠️

# SEMPAL

Audio sample triage tool built with Rust and egui.

[![Windows Support](https://img.shields.io/badge/Windows-0078D6?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/PORTALSURFER/sempal/releases)
[![Ubuntu Support](https://img.shields.io/badge/Linux-E95420?style=for-the-badge&logo=linux&logoColor=white)](https://github.com/PORTALSURFER/sempal/releases)
[![Mac Support](https://img.shields.io/badge/MACOS-adb8c5?style=for-the-badge&logo=macos&logoColor=white)](https://github.com/PORTALSURFER/sempal/releases)



[![Build release assets](https://github.com/PORTALSURFER/sempal/actions/workflows/release-build.yml/badge.svg)](https://github.com/PORTALSURFER/sempal/actions/workflows/release-build.yml)
[![GitHub package.json version](https://img.shields.io/github/v/release/PORTALSURFER/sempal?color=%40&label=latest)](https://github.com/PORTALSURFER/sempal/releases/latest)
![GitHub issues](https://img.shields.io/github/issues-raw/PORTALSURFER/sempal)
![GitHub all releases](https://img.shields.io/github/downloads/PORTALSURFER/sempal/total)
![Github license](https://img.shields.io/github/license/PORTALSURFER/sempal)
---

[![Sample workflow preview](assets/example.gif)](assets/example.gif)

[![Buy Me A Coffee](https://img.buymeacoffee.com/button-api/?text=Buy%20me%20a%20coffee&slug=portalsurfer&button_colour=FFDD00&font_colour=000000&font_family=Inter&outline_colour=000000&coffee_colour=ffffff)](https://buymeacoffee.com/portalsurfer)

## Getting Started

⚠️ **Warning:** Early alpha software. Some actions are destructive. Keep backups and proceed with caution.

Start at the `Sources` panel. Add a folder from your sample library. SEMPAL takes a moment to analyse the contents of the folder, creates a database in the folder indexing the samples and performs the similary prep. The `Sources` panel should now show the added source. If the source folder contains subfolders, they are listed in the `Folders` panel. See the [Hotkeys](#hotkeys) section for the shortcuts that allow switching focus between the panels. The Samples panel should now list the samples from the selected folder/source. 

## Hotkeys

`G` is the chord key, used with specific keys to focus on specific panels. 

| Hotkey  | Function |
| ------------- | ------------- |
| `G` `S`  | Focus sources list |
| `G` `T`  | Focus folder tree |
| `G` `B`  | Focus sample browser |
| `G` `W`  | Focus waveform  |
| `X`  | Toggle folder select|
| `Shift`+`F`  | Toggle find similar |

## Focus aware arrow key actions

Arrow keys perform various actions based on the panel in focus.

### Sources panel

`Arrow Up` & `Arrow Down` are used to switch between sources. Selection is non-cyclic, `Arrow Down` will not cycle back to the top of the list once the end of the list is reached.

### Folders panel

`Arrow Up` & `Arrow Down` are used to switch between folders. 
`Arrow Right` & `Arrow Left` are used to expand and collapse folders to show subfolders.

Symbols prefixed to the folder name :
```
. The source folder
> Folders with one or more subfolders
- Folders with no subfolders
```
Navigating with the keys only selects the folder. The list of samples displayed in the sample browser does not change. Use the hotkey `X` to toggle folder selection, which filters the list in the sample browser to the selected folder. 

### Sample browser panel

`Arrow Up` & `Arrow Down` are used to switch between the samples.
`Arrow Left` is used to focus the previously focused sample. 
`Arrow Right` is used to focus the next focused sample.
`Arrow Left` & `Arrow Right` functions are not limited to the currently selected source. If the previously focused sample is from another source, `Arrow Left` will focus on the previously focused sample and indicate that a different source is now focused.

### Waveform panel

`Arrow Left` & `Arrow Right` are used to slide selection left/right.
`Shift` + `Arrow Left` & `Shift` + `Arrow Right` are used to nudge selection left/right.

[Usage](/docs/usage.md) explain various function in detail. 

# For Developers

## Build from source

- Requires Rust (stable toolchain) and `cargo`.
- From the project root: `cargo run --release`.
- Or build once and run the binary: `cargo build --release` then `target/release/sempal`.
- Playback uses your default audio output device.
- Windows (ASIO): If you want to build with ASIO support (or your build fails looking for the ASIO SDK), download the Steinberg ASIO SDK and set `CPAL_ASIO_DIR` to the SDK path (e.g. a folder named `ASIOSDK`) before running `cargo build`/`cargo run`.
- Publishing a release triggers a workflow that regenerates `CHANGELOG.md` via `git-cliff` and opens a PR (since `main` is protected).


## ML model setup (PANNs)

- The app uses a PANNs model (burnpack) and requires the ONNX model to build the burnpack.
- Download + export the model once (Windows PowerShell):
  - `.\scripts\setup_panns.ps1`
- Or with Python directly:
  - `python .\scripts\setup_panns.py`
- By default, this writes to `%APPDATA%\\.sempal\\models`:
  - `panns_cnn14_16k.onnx`
  - `panns_cnn14_16k.bpk`
- Override locations if needed:
  - `SEMPAL_PANNS_ONNX_PATH` for the ONNX path
  - `SEMPAL_MODELS_DIR` for the models directory
- When downloading the ONNX model, provide an allowlisted HTTPS URL and its SHA-256 hash:
  - `SEMPAL_PANNS_ONNX_URL` and `SEMPAL_PANNS_ONNX_SHA256`
  - Override allowed hosts with `SEMPAL_PANNS_ONNX_ALLOWED_HOSTS` (comma-separated)

## Code style and linting

- Install tooling once: `rustup component add rustfmt clippy`.
- Format locally: `cargo fmt --all`.
- Check formatting (same as CI): `cargo fmt --all -- --check`.
- Lint locally (same as CI): `cargo clippy --all-targets`.
- CI runs `rustfmt`, `clippy`, and `cargo test` on Ubuntu/Windows/macOS for every push to `main`/`next` and all pull requests targeting those branches.

## Configuration and data

- Each source folder gets a hidden `.sempal_samples.db` that tracks indexed `.wav` files and their tags.
- App files live in a single `.sempal` folder inside your OS config directory:
  - Linux: `$HOME/.config/.sempal/config.toml`
  - Windows: `%APPDATA%\\.sempal\\config.toml`
  - macOS: `~/Library/Application Support/.sempal/config.toml`

## SQLite extensions (optional)

- Sempal can load a SQLite extension for faster vector operations via `SEMPAL_SQLITE_EXT`.
- Loading is opt-in with `SEMPAL_SQLITE_EXT_ENABLE=1` and restricted to `<app_root>/sqlite_extensions`.
- Unsafe mode (`SEMPAL_SQLITE_EXT_UNSAFE=1`) bypasses the allowlist, but it is ignored unless the build enables the `sqlite-ext-unsafe` cargo feature.
- If you need unsafe mode, rebuild with `cargo build --release --features sqlite-ext-unsafe` and supply a fully trusted extension path.

## Logging

- Startup initializes console logging and a per-launch log file under the same `.sempal` folder:
  - Linux: `$HOME/.config/.sempal/logs`
  - Windows: `%APPDATA%\\.sempal\\logs`
  - macOS: `~/Library/Application Support/.sempal/logs`
- Log filenames include the launch timestamp, and the 10 most recent files are retained by pruning the oldest.

## Documentation

- [Usage guide](docs/usage.md)
