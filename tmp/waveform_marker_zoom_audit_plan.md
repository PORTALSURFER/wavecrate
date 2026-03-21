# Waveform Marker Zoom Audit

Scope: playback-selection and edit-selection marker painting/resizing while the
native waveform view is zooming in/out.

Status legend:
- `[x]` done
- `[ ]` pending

## Findings (ROI order)

### [x] Keep off-plot resize drags stable while zoom changes
- Classification: `Bug fix`
- Confidence: high
- ROI: high
- Effort: medium
- Why it mattered:
  Anchor-based waveform drags clamped to the current viewport edge whenever the
  pointer moved outside the waveform plot. If the user then zoomed without
  moving the pointer back inside, the active selection edge could rebind to the
  newly zoomed viewport edge and appear to jump to a different absolute
  position.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/input/waveform_routing/drag.rs`
    previously recomputed `waveform_position_micros_from_point(...)` against the
    current view on every drag update.
  - `src/app/controller/playback/waveform_actions/selection_updates.rs`
    previously preserved the exact endpoint only when it still matched the
    *current* view edge.
  - New regression coverage:
    - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs`
    - `src/app/controller/playback/waveform_actions/selection_updates.rs`
- Recommended change:
  Capture a stable off-plot boundary lock in native drag state and preserve the
  exact locked endpoint through BPM snapping while the pointer remains outside
  the plot.
- Expected impact:
  Playback-selection resize, smart-scale resize, and edit-selection resize stay
  visually stable during concurrent zoom changes.
- Risks / tradeoffs:
  Off-plot drags now stay pinned to the first exact clamped endpoint until the
  pointer re-enters the plot, favoring stability over dynamic viewport-edge
  rebinding.
- Suggested validation:
  - `cargo test -p radiant waveform_resize_drag_keeps_outside_plot_lock_across_zoom_changes --lib -- --test-threads=1`
  - `cargo test -p sempal preserve_view_edge_keeps_exact_clamped_resize_endpoint --lib -- --test-threads=1`
  - `powershell -ExecutionPolicy Bypass -File scripts/ci_agent.ps1`

## Open Questions / Missing Definitions

- None. The conservative fix was to prefer drag stability over reinterpreting an
  unchanged off-plot pointer against a changing viewport.

## Rejected Ideas

- Disable waveform wheel zoom while a resize drag is active.
  - Considered because it would avoid the jump entirely.
  - Rejected because it removes a useful interaction path instead of making the
    existing behavior stable.
