# Advanced Waveform Tools

The waveform toolbar contains controls for tighter edits, loop checking, and analysis-assisted browsing.

## Zoom and View

- `Z` zooms to the play selection.
- `X` zooms out when the waveform is zoomed in.
- `Shift-X` zooms out with a silence margin.
- Use the waveform drag handle to drag the loaded sample out when drag-out is supported.

The same `X` key can toggle the focused sample selection when the waveform is already fully zoomed out and the browser is in that browsing context.

## Editmark Effects

Editmarks are the deliberate target for edit effects.

- Drag with the secondary mouse button to create an editmark.
- Use editmark handles to adjust fade and gain shape.
- Press `Enter` or click **Apply** to apply pending editmark effects.
- Use `Command-Z` and redo shortcuts to step through committed edit transactions.

Preview changes while dragging are grouped into one undoable action when the gesture is committed.

## Zero-Crossing Snap

Zero-crossing snap helps place selection edges on quieter waveform crossings.

Enable it when trimming clicks or making loop boundaries. Disable it when you need fully free selection placement.

## Similar Sections

Similar Sections looks for regions in the loaded waveform that resemble the current play selection.

1. Create a play selection.
2. Toggle Similar Sections from the toolbar.
3. Review highlighted matching regions in the waveform.

Use it to find repeated hits, loops, or similar phrases inside longer recordings.

## Beat Guides and Metronome

Beat guides draw divisions over the waveform.

- Toggle beat guides from the toolbar.
- Set the beat guide count in the number field.
- Toggle BPM Snap to make playmark and editmark resizing land on whole BPM values derived from the beat guide count.
- Toggle the metronome when you want click feedback during playback.

Beat guides are visual timing aids. They do not turn Wavecrate into an arrangement editor.

## Random Playback Controls

The random toolbar button supports several audition styles.

- Click random to play a random section.
- Shift-click random to play a random listed sample range.
- Command-click random to toggle sticky random playback.
- `Option-Space` or `Control-Space` plays a random sample section.

Sticky random playback makes `Space` keep choosing random sections until you turn sticky random off.
