---
layout: default
title: Usage
permalink: /usage
description: How to set up Wavecrate, triage samples, edit waveforms, and manage sources.
---

# Wavecrate Usage Guide

* TOC
{:toc}

## Documentation status

This is the current lightweight GitHub Pages usage guide. The expanded public
docs live at <https://portalsurfer.org/wavecrate/docs/>; both surfaces document
the same shipped app platforms: macOS and Windows.

## Quick start
- Add a source folder with **+** or by dropping it onto the Sources panel; the first `.wav` row auto-loads and starts playback.
- Drag on the waveform with the primary mouse button to create a play mark selection, or use the secondary mouse button to create an edit mark selection for destructive in-place edits (crop, trim, reverse, fades, mute, smooth, normalize).
- Drag the selection handle onto Samples or a folder to export a trimmed clip (saved into the selected folder when applicable).
- Use the browser filter chips to triage quickly: `All/Keep/Trash/Untagged` for tags and `NVR/1M/1W` for playback age. `Space` toggles play/pause, `Esc` stops playback (or exits slice review first), and `Silence Split` opens a keyboard slice-review loop.
- Use **Find similar** or the **Similarity map** tab to explore related samples after similarity prep finishes.

## Layout at a glance
- **Sources (left):** Source list plus folder browser + selected folders; missing sources show `!`.
- **Center:** Waveform viewer (mono/split, loop, BPM snap) above the Samples list or the Similarity map tab.
- **Top bar:** Options menu (export/trash/audio/analysis), update status, volume slider, and background task progress.
- **Resizable sidebar:** Drag the divider to resize Sources and the main view.

## Configuration and storage
- Wavecrate app builds currently support macOS and Windows. Linux is not currently supported for app installs.
- App files live in a single `.wavecrate` folder inside your supported OS config directory. You can override the base dir with `WAVECRATE_CONFIG_HOME`.
  - Windows: `%APPDATA%\\.wavecrate\\`
  - macOS: `~/Library/Application Support/.wavecrate/`
- App settings live in `config.toml`; sources are stored in `library.db` in the same folder. Legacy `config.json` files migrate automatically.
- You can override the app data root by setting `app_data_dir` in `config.toml` (absolute path to the `.wavecrate` folder). This controls where models, logs, and the library DB live.
- Each source keeps `.wavecrate_samples.db` beside the audio. Logs live under `.wavecrate/logs`.
- Downloaded bundles may include ML assets under `models/`; keep those assets with the extracted app bundle unless a release note says they should be moved into app data.
- Model assets (when present) live in `.wavecrate/models` (Windows: `%APPDATA%\\.wavecrate\\models\\`, macOS: `~/Library/Application Support/.wavecrate/models/`).
- Tip: Use **Options → Open config folder** to jump to the right place on disk.

## Manage sources
- Click **+** or drop a folder to add. Wavecrate creates/uses `.wavecrate_samples.db` and loads `.wav` entries.
- Right-click a source row: **Quick sync**, **Hard sync (full rescan)**, **Prepare similarity search**, similarity prep options, **Open in file explorer**, **Remap source...**, **Remove source**. Add new files outside Wavecrate? Run a sync.
- Selecting any row loads the waveform and (by default) starts playback. Missing sources are prefixed with `!`. Similarity prep starts automatically after adding a source.
- Use the **Folders** tree to filter which subfolders appear in the Samples list. Drag samples onto folders to move files on disk.

## Browse and triage
- Triage chips (`All/Keep/Trash/Untagged`) change the visible list. Rows show number columns and right-edge keep/trash markers.
- Playback-age chips use the current native toolbar labels: `NVR` (never played), `1M` (last played at least 30 days ago), and `1W` (last played at least 7 days ago but less than 30 days ago). Current behavior lets you combine these chips; `Alt + click` on one chip inverts the selection to the other playback-age chips.
- Search box performs fuzzy matching within the current filter; clear to restore the full list.
- Dice button in the browser toolbar: click 🎲 to play a random visible sample; **Shift + click** toggles sticky random navigation (same as `Alt + R`).
- Selection basics: click to focus; **Shift + click** extends; **Ctrl/Cmd + click** toggles multi-select while keeping focus. **Up/Down** moves focus; **Shift + Up/Down** extends. Toggle **Alt + R** to lock random navigation so **Down** plays random visible samples and **Up** steps backward through random history.
- Tagging: **Right Arrow** → Keep (Trash → Neutral, others → Keep). **Left Arrow** → Trash (Keep → Neutral, others → Trash). **Ctrl/Cmd + Right/Left** moves the selection across triage columns.
- Row context menu: **Open in file explorer**, **Find similar**, Tag Keep/Neutral/Trash, **Normalize (overwrite)**, **Rename**, **Delete file**. Applies to the focused row or multi-select.
- **Ctrl/Cmd + C** copies the focused or selected sample file(s) to the clipboard as file drops (for DAWs/file managers). Dragging a row into the browser retags it to the active filter (All/Untagged → Neutral).

## Similarity search & map
- **Prepare similarity search** from a source row menu or the **Prepare similarity map** button in the map tab (runs analysis + embeddings + clustering for that source).
- **Find similar** filters the Samples list to related items; use **Clear similar** to return to the full list.
- **Similarity map** tab: scroll to zoom, right-drag to pan, hover or click a point to preview and focus that sample.

## Playback and waveform editing
- **Space** toggles play/pause. **Ctrl/Cmd + Space** plays from the waveform cursor (falls back to play/pause). **Shift + Space** replays from the last start.
- **Esc** exits slice review first when active. Otherwise it stops playback; if already stopped, it clears browser + waveform selection/cursor/folder selection. Click waveform to seek; clicking while a selection exists clears it.
- **Loop on/off** uses the selection when present, otherwise the full file; a loop bar shows the active region and the playhead. Toggle via hotkey `L` or the **Loop: On/Off** button above the waveform. **Shift + L** (or Shift + click the loop button) enters locked loop override mode: the first press forces **locked on**, repeated Shift presses/clicks flip between **locked on** and **locked off**, and plain `L`/click exits lock mode by performing the normal toggle for the current sample.
- Waveform controls: **Mono envelope** (downmixed average) / **Split L/R**, **Loop: On/Off**, **BPM snap** with a BPM input.
- Drag with the primary mouse button to create a play mark selection for auditioning, looping, extraction, and DAW handoff. Drag with the secondary mouse button to create an edit mark selection for destructive in-place edits. Drag edge brackets to resize. When **BPM snap** is enabled, playback-selection drags snap to beats by default; hold **Alt** while dragging to move freely, and hold **Shift** on a playback edge drag to smart-scale BPM from the resized span. Mouse wheel zooms; **Shift + wheel** pans when zoomed. The bottom handle supports drag-and-drop.
- Click **Silence Split** to detect slices and enter review immediately. Use **Left/Right** to move between slices, **Space** to audition the focused slice, **A** to mark or unmark it for export, **E** to export the marked slices, and **Esc** to leave review mode without clearing the slice batch.
- Right-click a selection for destructive in-place edits that overwrite the source: **Crop to selection**, **Trim selection out**, **Reverse selection**, **Fade to null** (L→R or R→L), **Mute selection**, **Remove clicks** (best for short selections with audio on both sides), **Normalize selection** (adds 5 ms edge fades). Wavecrate uses confirmations and undo-oriented recovery where it owns the operation, but keep backups and choose deliberately.
- Destructive edits prompt for confirmation; enable **Yolo mode** in **Options** to apply without prompting.
- Drag the selection handle:
  - Onto the Samples list to save a trimmed clip tagged by the current filter (saved into the selected folder when applicable).
  - Onto a folder in the Sources panel to save into that folder (same source only).
  - On Windows, dragging outside the window exports and starts an external drag for DAWs/file managers.

## Trash and cleanup
- Open **Options** in the status bar to set or open the trash folder.
- **Move trashed samples to folder:** Moves all Trash-tagged samples from every source into the trash folder (keeps relative paths) and removes them from lists.
- Hotkey: Press `P` or `Shift + P` from anywhere to trigger **Move trashed samples to folder** (uses the configured trash folder and existing confirmation).
- **Take out trash:** Permanently deletes everything inside the trash folder.

## Drag, drop, and clipboard tips
- Drop folders onto the Sources panel to add them.
- Drag sample rows back into the browser (for retagging), or onto folders to move files.
- Drag selections or samples outside the window on Windows to start an external drag-out. Use **Ctrl/Cmd + C** to copy the current waveform selection as an exported wav clip when waveform focus is active, or to copy selected sample rows as file drops when browser focus is active.

## Hotkeys (focus-aware)
- **Global:** `Space` play/pause; `Ctrl/Cmd + Space` play from cursor; `Shift + Space` replay from last start; `Esc` stop playback / clear selection; `Ctrl/Cmd + Z` or `U` undo; `Ctrl/Cmd + Y` or `Shift + U` redo; `L` toggle loop; `Shift + L` enter/cycle locked loop override; `P` or `Shift + P` move trashed samples to the trash folder; `[` trash selected sample(s); `]` keep selected sample(s); `'` tag selected sample(s) as neutral; `Shift + R` play a random visible sample and auto-play; `Alt + R` toggle sticky random navigation; `Ctrl/Cmd + Shift + R` step backward through random history; `Ctrl/Cmd + Shift + L` copy status log; `Ctrl/Cmd + /` toggle hotkey overlay; `Shift + F1` submit a GitHub issue (connect GitHub first); `F11` toggle maximized window; focus chords (press `G` then): `W` waveform, `S` sample browser, `Shift + S` sources list.
- **Sample browser focus:** `Up/Down` move (or jump randomly when sticky mode is on); `Shift + Up/Down` extend; `Right Arrow` Keep; `Left Arrow` Trash; `Ctrl/Cmd + Right/Left` move across triage columns; `Ctrl/Cmd + C` copy focused/selected sample file(s); `X` toggle selection; `Ctrl/Cmd + A` select all; `N` normalize (overwrite); `D` delete.
- **Waveform focus:** `Space` auditions the focused slice while slice review is active; `Left/Right` move between review slices or slide the current selection when review is inactive; `Shift + Left/Right` nudge the current selection; `Ctrl/Cmd + C` copy the current selection as an exported wav clip; `A` marks the focused review slice for export; `E` exports the marked review slices or the current selection/slice batch and marks the new file(s) keep x1; `Shift + E` exports the same clip(s) and immediately marks them keep x2; `Esc` exits slice review before other waveform clearing behavior; `C` crop selection (overwrite), `Shift + C` crop selection as new sample; `T` trim selection; `\\` fade selection (left to right); `/` fade selection (right to left); `M` mute selection / merge selected slices; `N` normalize selection/sample.
