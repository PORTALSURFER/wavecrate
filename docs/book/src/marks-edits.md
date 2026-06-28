# Playmarks, Editmarks, and Edits

Wavecrate has two selection concepts on the waveform: playmarks for auditioning and handoff, and editmarks for destructive editing.

## Playmarks

Drag with the primary mouse button to create a playmark selection.

Use playmarks for:

- auditioning a short region
- looping a useful moment
- extracting a clip
- copying or dragging a selected region to another app

Playmarks are fast and musical. They are the default selection you make while listening.

## Editmarks

Drag with the secondary mouse button to create an editmark selection.

Use editmarks when you want the selected range to be the explicit target for edits such as crop, trim, mute, reverse, fade, or normalize.

When both a playmark and editmark exist, edit commands generally prefer the editmark because it is the more deliberate editing target.

## Common Edits

- **Extract:** write the selected region as a new WAV.
- **Crop:** keep the selected region and remove everything else.
- **Trim:** remove the selected region and close the gap.
- **Mute:** silence the selected region without changing duration.
- **Reverse:** reverse the selected region in place.
- **Fade:** fade the selected region to silence from either direction.
- **Normalize:** adjust level for the selected region or file.

Destructive edits prompt for confirmation unless Yolo mode is enabled. Keep backups when working on important material.

## Visual Feedback

After edits, the waveform should reload or update so the visible audio reflects the change. If the visual state looks stale after an edit, try selecting the file again or reopening the source, then include the exact action sequence and logs when reporting it.
