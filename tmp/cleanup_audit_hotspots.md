# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-04-02T21:06:06Z`
- Branch: `next`
- Commit: `79fb5c7f`
- Rust files scanned: 1431
- File-size budget limit: `400`

## Summary

- Over file-size budget: 8
- Function spans captured: 8788
- Files with `dead_code` suppressions: 3
- Files with `clippy::too_many_arguments` suppressions: 3
- Likely large-file test-gap hotspots (heuristic): 130

## Largest Rust files

| Lines | File |
| --- | --- |
| 1636 | `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` |
| 1133 | `vendor/radiant/src/app/hotkeys.rs` |
| 795 | `vendor/radiant/src/app/actions/mod.rs` |
| 763 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` |
| 713 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` |
| 564 | `src/app_core/actions/catalog/kinds.rs` |
| 475 | `src/app/controller/playback/transport/selection.rs` |
| 407 | `src/selection/range.rs` |
| 400 | `src/app/controller/playback/persistent_waveform_cache.rs` |
| 399 | `tests/unit/audio_output_tests.rs` |
| 397 | `vendor/radiant/src/gui/native_shell/state/motion_overlay.rs` |
| 397 | `src/sqlite_ext.rs` |
| 396 | `vendor/radiant/src/gui/native_shell/state/tests/selection_states.rs` |
| 396 | `src/app_core/native_bridge/metrics.rs` |
| 395 | `vendor/radiant/src/gui_runtime/native_vello/runtime_events/pointer.rs` |
| 394 | `src/app/controller/library/source_folders/delete_recovery/journal.rs` |
| 394 | `src/app/controller/library/selection_edits/ops.rs` |
| 393 | `src/issue_gateway/token_store/tests.rs` |
| 393 | `src/app/controller/tests/drag_drop_folders/sample_moves.rs` |
| 392 | `vendor/radiant/src/gui_runtime/native_vello.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| --- | --- |
| 635 | `is_active` (`vendor/radiant/src/app/hotkeys.rs:153`) |
| 325 | `render_browser_frame` (`vendor/radiant/src/gui/native_shell/state/frame_build/browser/panel.rs:3`) |
| 288 | `render_browser_rows_window` (`vendor/radiant/src/gui/native_shell/state/frame_build/browser/rows.rs:3`) |
| 275 | `waveform_toolbar_buttons` (`vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs:9`) |
| 262 | `redraw` (`vendor/radiant/src/gui_runtime/native_vello/runtime_render/present.rs:81`) |
| 251 | `build_chrome_motion_overlay_into` (`vendor/radiant/src/gui/native_shell/state/motion_overlay.rs:59`) |
| 250 | `representative_action_for_kind` (`src/app_core/actions/catalog/entries.rs:47`) |
| 246 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests/dispatch/core.rs:49`) |
| 238 | `waveform_drag_action_clamps_and_preserves_selection_anchor` (`vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs:478`) |
| 227 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 210 | `sizing_for_tier` (`vendor/radiant/src/gui/native_shell/style/sizing.rs:12`) |
| 206 | `action_slug` (`vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98`) |
| 190 | `render_browser_focus_overlay` (`vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs:201`) |
| 188 | `compute_browser_toolbar_sections` (`vendor/radiant/src/gui/native_shell/layout_adapter/controls/browser_toolbar.rs:27`) |
| 185 | `build_with_style_and_runtime` (`vendor/radiant/src/gui/native_shell/layout/mod.rs:116`) |
| 181 | `folder_browser_group` (`vendor/radiant/src/gui/native_shell/state/automation/sidebar.rs:134`) |
| 175 | `render_confirm_prompt` (`vendor/radiant/src/gui/native_shell/state/overlays/prompt.rs:6`) |
| 174 | `push_waveform_playhead_overlay` (`vendor/radiant/src/gui/native_shell/state/waveform_segments/overlay.rs:8`) |
| 171 | `render_status_bar` (`vendor/radiant/src/gui/native_shell/state/frame_build/status_bar.rs:5`) |
| 169 | `emit_edit_fade_overlays` (`vendor/radiant/src/gui/native_shell/state/waveform_segments/fades.rs:9`) |

## Over file-size budget

| Lines | File |
| --- | --- |
| 1636 | `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` |
| 1133 | `vendor/radiant/src/app/hotkeys.rs` |
| 795 | `vendor/radiant/src/app/actions/mod.rs` |
| 763 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` |
| 713 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` |
| 564 | `src/app_core/actions/catalog/kinds.rs` |
| 475 | `src/app/controller/playback/transport/selection.rs` |
| 407 | `src/selection/range.rs` |

## dead_code suppression density

| Occurrences | File |
| --- | --- |
| 8 | `vendor/radiant/src/gui/layout_core/model.rs` |
| 1 | `vendor/radiant/src/gui/native_shell/state/tests/browser_scrollbars/folder_waveform.rs` |
| 1 | `src/lib.rs` |

## too_many_arguments suppression density

| Occurrences | File |
| --- | --- |
| 2 | `vendor/radiant/src/gui/layout_core/engine/layout.rs` |
| 1 | `vendor/radiant/src/gui/layout_core/engine/layout/scroll_linear.rs` |
| 1 | `vendor/radiant/src/app/sources.rs` |

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.
Skips dedicated test modules/paths (`tests/**`, `tests.rs`, `*_test.rs`, `*_tests.rs`) and sibling module tests declared through `mod.rs` + `tests.rs`.

| Lines | File |
| --- | --- |
| 564 | `src/app_core/actions/catalog/kinds.rs` |
| 390 | `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs` |
| 390 | `src/app/controller/jobs/file_ops_types.rs` |
| 386 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 385 | `src/app/controller/library/wavs/browser_facade.rs` |
| 381 | `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs` |
| 364 | `src/gui_test/aiv/packs/cases/browser.rs` |
| 363 | `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` |
| 357 | `vendor/radiant/src/gui/native_shell/state/waveform_segments/fades.rs` |
| 355 | `src/app_core/native_shell/waveform_projection.rs` |
| 354 | `vendor/radiant/src/gui/native_shell/state/frame_build/browser/panel.rs` |
| 354 | `src/app/controller/library/browser_controller/helpers.rs` |
| 352 | `vendor/radiant/src/app/sources.rs` |
| 351 | `vendor/radiant/src/gui_runtime/native_vello/input/waveform_handles.rs` |
| 351 | `src/app/state/sources.rs` |
| 349 | `src/app/controller/gui_fixtures.rs` |
| 348 | `vendor/radiant/src/gui_runtime/native_vello/text_runtime.rs` |
| 346 | `vendor/radiant/src/gui/native_shell/state/automation/browser.rs` |
| 346 | `src/app/controller/library/source_folders/delete_recovery/restore_merge/ops.rs` |
| 345 | `src/app/controller/library/background_jobs/polling/library_handlers.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
