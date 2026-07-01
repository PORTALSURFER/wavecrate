# Getting Started

## Install

1. Open the Wavecrate download page.
2. Download the latest macOS or Windows stable, RC, or nightly bundle.
3. Unzip the bundle.
4. Launch Wavecrate.
5. Add a sample folder from the Sources panel.

Wavecrate works with ordinary local folders. It does not upload your samples or require a cloud library.

Wavecrate app builds currently support macOS and Windows. Linux is not currently supported for app installs.

> Safety warning: Wavecrate includes destructive actions that can modify, rename, move, or delete files when you choose those commands. Keep backups for important sample folders.

## Add Your First Source

Use the plus button in the Sources panel, or drag a folder onto the Sources panel. Wavecrate indexes supported audio files and stores source metadata beside the source so browsing is fast after the first pass.

After a source is added:

- Select folders in the left panel to narrow the visible sample list.
- Select a sample row to load and audition it.
- Use filters, search, tags, ratings, and collections to narrow what you hear.

## First Five-Minute Workflow

1. Add a folder with a small set of WAV files.
2. Click a sample row and press `Space` to play or pause.
3. Press `Down` to move through the visible list.
4. Drag across the waveform to create a playmark selection.
5. Press `L` to loop the selected region.
6. Press `E` to extract a useful selected region.
7. Rate or tag the result so you can find it later.

## Where Wavecrate Stores Settings

Wavecrate keeps app data in a `.wavecrate` folder inside your supported operating system config directory.

- macOS: `~/Library/Application Support/.wavecrate/`
- Windows: `%APPDATA%\\.wavecrate\\`

Use **Options -> Open config folder** in the app when you need logs, settings, or support context.
