# Native Shell Legacy Baseline Contract

This document captures the target geometry and copy contract for the current
`radiant` native shell compatibility layer while we migrate from the legacy
`egui` renderer.

The native shell is legacy compatibility infrastructure, not the preferred
core API for new generic Radiant work. New reusable layout, widget, and
runtime work should target `radiant::layout`, `radiant::widgets`, and
`radiant::runtime` instead.

## Reference viewport

- Viewport: `1440x810`
- Sidebar width: `155..=220`
- Waveform card height: `150..=280`
- Browser visible-row capacity: `>=22`
- Top bar height: `<=34`
- Status bar height: `<=20`

These constraints are validated by native-shell layout/style tests in
`vendor/radiant/src/gui/native_shell/mod.rs` and
`vendor/radiant/src/gui/native_shell/style.rs`.

## Browser chrome copy contract

Browser tab/toolbar/footer text is projected by the host and should not rely on
renderer hardcoded labels:

- tab labels (`Samples`, `Similarity map`)
- search prefix + placeholder
- activity labels (ready/busy)
- sort prefix + order label
- map similarity mode label
- browser item-count footer label

## Waveform chrome copy contract

Waveform metadata copy is projected by the host and should include:

- transport hint label
- tempo label
- zoom label
- playhead/cursor/view labels

This keeps layout/hit-testing deterministic while allowing host-controlled copy
for legacy parity tuning.
