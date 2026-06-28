# Troubleshooting

## Find Logs

Use **Options -> Open config folder** and look under `.wavecrate/logs`.

Wavecrate alpha app builds currently support macOS and Windows. On a normal supported install, the config folder is:

- macOS: `~/Library/Application Support/.wavecrate/`
- Windows: `%APPDATA%\\.wavecrate\\`

## Common Problems

### A source disappeared

The drive or folder may be disconnected. Reconnect it, then use source sync or remap the source if the path changed.

### New files do not appear

Run source sync from the source context menu. Use hard sync when ordinary sync does not pick up external changes.

### Similarity search is empty

Prepare similarity data for the source first. Similarity search depends on analysis, embeddings, and clustering work that can take time on large folders.

### A destructive edit changed the wrong file

Stop editing, keep the current folder as-is, and gather logs. If the source should be protected, make sure it is configured as a protected or locked source before continuing.

### The waveform looks stale

Select another sample and return, or restart the app. If stale visuals repeat, include the exact action sequence and logs in the bug report.
