# Native Bridge Projection Cache

This note documents the retained-model contract implemented in
`src/app_core/native_bridge.rs` and `src/app_core/native_bridge/**`.

Use it when changing:

- native-shell app model projection
- derived dirty-source invalidation
- projection-key composition
- bridge profiling or snapshot-assertion workflows

## Retained projection model

The native bridge keeps a retained `NativeAppModel` cache so most pulls can
reuse previously projected segments instead of rebuilding the full model on
every action.

Core flow:

1. `SempalNativeBridge::pull_model_arc_snapshot` prepares controller state and
   flushes derived invalidation unless a local-only fast path is active.
2. `projection_key_snapshot_for_pull` resolves the current full
   `NativeProjectionCacheKey`.
3. `DerivedProjectionState::from_controller_with_app_key` derives the segment
   keys used by retained projection.
4. `NativeProjectionCache::resolve_or_project_with_derived` either returns the
   cached model on a full-key hit or rematerializes only the dirty segments.

Primary implementation references:

- `src/app_core/native_bridge.rs`
- `src/app_core/native_bridge/projection_cache.rs`
- `src/app_core/native_bridge/projection_cache/projection_key.rs`
- `src/app_core/native_bridge/projection_cache/segment_materialize.rs`

## Segment keys and ownership

The retained cache is partitioned into five explicit segments plus one static
non-segment bucket.

- `StatusBar`
  - Key: `StatusProjectionCacheKey`
  - Built by: `build_status_projection_key`
  - Covers status/footer text plus browser-selection summary fields.
- `BrowserFrame`
  - Key: `BrowserFrameProjectionCacheKey`
  - Built by: `build_browser_frame_projection_key`
  - Covers browser chrome, action cluster, visible-count metadata, sort/tab,
    focused sample label, and active rating filters.
- `BrowserRowsWindow`
  - Key: `BrowserRowsProjectionCacheKey`
  - Built by: `build_browser_rows_projection_key`
  - Covers only the materialized browser row window and selection markers.
- `MapPanel`
  - Key: `MapProjectionCacheKey`
  - Built by: `build_map_projection_key`
  - Covers map open state, pan/zoom, hover/selection revisions, and dataset
    revisions.
- `WaveformOverlay`
  - Key: `WaveformProjectionCacheKey`
  - Built by: `build_waveform_projection_key`
  - Covers waveform image signature, selection/edit-selection/fade geometry,
    view window, BPM/loop/channel-view toggles, and loaded-sample revision.
- `GLOBAL_STATIC`
  - Key: `NonSegmentStaticProjectionCacheKey`
  - Built by: `build_non_segment_static_projection_key`
  - Covers sources list, folder selection/search, update state, volume,
    transport, focus context, and browser column counts.

Important rule:

- `NativeProjectionCacheKey` is the whole-model short-circuit key.
- Segment keys exist so a full-key miss can still reuse untouched retained
  segments instead of rebuilding the entire `NativeAppModel`.

Fields that are refreshed every pull and intentionally do not have dedicated
retained keys:

- `selected_column`
- progress overlay
- confirm prompt
- drag overlay

Those are refreshed by `refresh_non_segment_always_fields` and
`refresh_non_segment_overlay_fields` in
`src/app_core/native_bridge/projection_cache/segment_materialize.rs`.

## Invalidation boundaries

The bridge tracks derived dirty sources separately from retained projection
keys. Those layers are related, but not interchangeable.

Action-to-dirty-source routing lives in
`src/app_core/native_bridge/invalidation.rs`.

Key rules:

- `classify_dirty_source` maps high-frequency actions to one primary source:
  browser, map, transport, status, or waveform.
- `action_prefers_targeted_invalidation` keeps browser navigation/search
  actions off broad invalidation so map/transport/status keys do not churn for
  browser-only work.
- `action_requires_projection_cache_invalidation` identifies actions that still
  need broad invalidation when there is no safe targeted-only source update.
- `waveform_render_inputs_require_refresh` treats
  `DirtyReason::WaveformOverlayAction` as overlay-only. Cursor/selection/fade
  changes do not force a waveform image rerender, while view/BPM/zoom changes
  do.

Projection-cache invalidation rules:

- `NativeProjectionCache::invalidate()` clears the retained model and all keys.
  This is test-only and represents a full cold start.
- `NativeProjectionCache::invalidate_key_only()` clears only `app_key`.
  The retained model and per-segment keys stay alive so the next pull can run
  segment-by-segment reuse instead of rebuilding from scratch.
- `flush_derived_updates_before_pull` uses key-only invalidation when
  `DerivedNodeId::NativeAppProjectionKey` is dirty.
- Browser local-fast-path actions also use key-only invalidation after they
  compare the before/after projection-key snapshots.

## Local-only pull fast path

Some actions mutate only native-visible UI state and do not require the normal
`prepare_native_frame(false)` + derived-flush path on the very next pull.

Classification lives in `uses_local_model_pull_fast_path` in
`src/app_core/native_bridge/action_classification.rs`.

Current local-only actions:

- browser focus/view movement
- focus-surface switches
- opening/closing options panels
- prompt input edits

Execution contract:

- A mutating action compares the projection key before and after the local UI
  change.
- If the key changed, the bridge runs `invalidate_key_only()` and schedules
  `PendingModelPullPreparation::LocalOnly` for the next pull.
- `consume_local_model_pull_fast_path` allows that shortcut only when:
  - transport is not playing
  - no derived nodes are dirty
  - the bridge has not exceeded
    `LOCAL_MODEL_PULL_FAST_PATH_BURST_LIMIT` consecutive local-only pulls
- If any of those checks fail, the bridge falls back to the normal full
  preparation path and resets the burst counter.

This fast path is intentionally one-shot and conservative. It exists to reduce
bridge work for focus/navigation actions, not to bypass derived invalidation in
general.

## Waveform preview fast paths

The bridge also has a separate high-frequency waveform path:

- `PendingWaveformActions` coalesces seek/cursor/selection/edit/zoom actions
  per pull frame.
- `flush_pending_waveform_actions` emits those actions in deterministic order
  before projection.
- Overlay-only waveform edits dirty `DerivedNodeId::WaveformState` with
  `DirtyReason::WaveformOverlayAction`, which avoids unnecessary waveform-image
  refreshes.

Immediate overlay preview is controlled by
`SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW`. This env var is independent
of the retained projection metrics feature.

## Profiling and assertion env vars

Bridge profiling hooks are compiled in only when the
`native-bridge-metrics` cargo feature is enabled.

- `SEMPAL_NATIVE_BRIDGE_PROFILE`
  - Enables periodic bridge metrics logging.
  - Logging cadence is controlled by `BRIDGE_PROFILE_INTERVAL`
    (`240` pull calls in the current implementation).
- `SEMPAL_NATIVE_BRIDGE_ASSERT_PROJECTION_SNAPSHOT`
  - Enables runtime validation that the cached projection-key snapshot still
    matches a freshly rebuilt key before projection.
  - When a stale snapshot is detected, the bridge records the mismatch and
    immediately replaces the cached snapshot with the fresh key.
- `SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW`
  - Controls whether overlay-preview waveform actions apply immediately instead
    of waiting for the queued waveform batch flush.
  - Default: enabled.

Metrics/probe entrypoints exposed from `src/app_core/native_bridge.rs`:

- `measure_projection_segment_lookup_counts`
- `measure_projection_segment_probe`
- `measure_projection_rebuild_cause_counts`

Use those helpers for repeatable local experiments instead of adding ad-hoc
logging inside the bridge.

## Test coverage anchor

Behavioral coverage for this contract lives primarily in
`src/app_core/native_bridge/tests.rs`.

That suite covers:

- full-model cache hits vs segment misses
- dirty-segment bit assignment
- local-only pull preparation transitions
- waveform overlay vs view invalidation behavior
- projection-key snapshot correction paths
