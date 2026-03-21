# Waveform Marker Zoom Audit

Scope: playback-selection and edit-selection marker painting/resizing while the
native waveform view is zooming in/out.

Status legend:
- `[x]` done
- `[ ]` pending

## Findings (ROI order)

### [x] Keep resize drags stable when zoom and drag updates interleave
- Classification: `Bug fix`
- Confidence: high
- ROI: high
- Effort: medium
- Why it mattered:
  Two interaction paths could reinterpret a resize drag against the wrong
  viewport and make the selection jump:
  1. Anchor-based waveform drags clamped to the current viewport edge whenever
     the pointer moved outside the waveform plot. If the user then zoomed
     without moving the pointer back inside, the active selection edge could
     rebind to the newly zoomed viewport edge and appear to jump to a different
     absolute position.
  2. Wheel zoom is reduced immediately into the bridge, but the native runner's
     cached `AppModel` view normally refreshes later during scene rebuild. If a
     new drag sample arrived before that rebuild, the runtime could still map
     the pointer through stale `view_start_micros/view_end_micros` bounds and
     emit a selection update for the wrong absolute time.
- Evidence:
  - `vendor/radiant/src/gui_runtime/native_vello/input/waveform_routing/drag.rs`
    previously recomputed `waveform_position_micros_from_point(...)` against the
    current view on every drag update.
  - `vendor/radiant/src/gui_runtime/native_vello/runtime_events/keyboard.rs`
    previously emitted waveform wheel zoom without refreshing the runner's local
    waveform view snapshot during an active drag.
  - `src/app/controller/playback/waveform_actions/selection_updates.rs`
    previously preserved the exact endpoint only when it still matched the
    *current* view edge.
  - New regression coverage:
    - `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs`
    - `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs`
    - `src/app/controller/playback/waveform_actions/selection_updates.rs`
- Recommended change:
  Capture a stable off-plot boundary lock in native drag state, refresh the
  runner's local waveform view snapshot immediately after wheel zoom during an
  active drag, and preserve the exact locked endpoint through BPM snapping while
  the pointer remains outside the plot.
- Expected impact:
  Playback-selection resize, smart-scale resize, and edit-selection resize stay
  visually stable during concurrent zoom changes.
- Risks / tradeoffs:
  Off-plot drags now stay pinned to the first exact clamped endpoint until the
  pointer re-enters the plot, favoring stability over dynamic viewport-edge
  rebinding. Active drag wheel zoom now performs one extra local model pull so
  subsequent drag samples use the updated viewport immediately.
- Suggested validation:
  - `cargo test -p radiant waveform_resize_drag_keeps_outside_plot_lock_across_zoom_changes --lib -- --test-threads=1`
  - `cargo test -p radiant waveform_wheel_zoom_refreshes_local_view_before_next_drag_sample --lib -- --test-threads=1`
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
