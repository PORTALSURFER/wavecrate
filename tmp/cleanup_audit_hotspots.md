# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-04-18T13:33:39Z`
- Branch: `next`
- Commit: `ca082e0f`
- Rust files scanned: 1464
- File-size budget limit: `400`

## Summary

- Over file-size budget: 48
- Function spans captured: 9453
- Files with `dead_code` suppressions: 3
- Files with `clippy::too_many_arguments` suppressions: 3
- Likely large-file test-gap hotspots (heuristic): 139

## Largest Rust files

| Lines | File |
| --- | --- |
| 1674 | `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` |
| 1133 | `vendor/radiant/src/app/hotkeys.rs` |
| 942 | `src/app/controller/library/wavs/browser_pipeline/tests.rs` |
| 856 | `vendor/radiant/src/app/actions/mod.rs` |
| 763 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` |
| 713 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` |
| 630 | `src/app/controller/jobs/messages.rs` |
| 622 | `src/app/controller/library/wavs/similar/resolve/repository.rs` |
| 610 | `src/app/controller/ui/file_ops.rs` |
| 587 | `src/app/controller/library/source_folders/actions/rename_move_delete.rs` |
| 580 | `src/app/controller/jobs/file_ops_types.rs` |
| 536 | `src/app_core/native_shell/tests/browser_cache.rs` |
| 533 | `vendor/radiant/src/gui/native_shell/state/tests/selection_states.rs` |
| 524 | `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` |
| 523 | `vendor/radiant/src/gui/native_shell/state/tests/overlay_controls/context_and_controls.rs` |
| 518 | `src/app/controller/library/browser_controller/helpers.rs` |
| 518 | `crates/sempal-library/src/sample_sources/db/mod.rs` |
| 506 | `src/app_core/native_shell/options_panel_projection.rs` |
| 503 | `src/app/controller/tests/source_async.rs` |
| 482 | `src/app/controller/library/source_folders/delete_recovery/journal.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| --- | --- |
| 635 | `is_active` (`vendor/radiant/src/app/hotkeys.rs:153`) |
| 333 | `render_browser_rows_window` (`vendor/radiant/src/gui/native_shell/state/frame_build/browser/rows.rs:3`) |
| 309 | `render_browser_frame` (`vendor/radiant/src/gui/native_shell/state/frame_build/browser/panel.rs:3`) |
| 275 | `waveform_toolbar_buttons` (`vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs:9`) |
| 262 | `redraw` (`vendor/radiant/src/gui_runtime/native_vello/runtime_render/present.rs:84`) |
| 253 | `build_chrome_motion_overlay_into` (`vendor/radiant/src/gui/native_shell/state/motion_overlay.rs:59`) |
| 246 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests/dispatch/core.rs:101`) |
| 238 | `waveform_drag_action_clamps_and_preserves_selection_anchor` (`vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs:478`) |
| 228 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 220 | `action_slug` (`vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98`) |
| 210 | `sizing_for_tier` (`vendor/radiant/src/gui/native_shell/style/sizing.rs:12`) |
| 205 | `render_browser_focus_overlay` (`vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs:206`) |
| 199 | `waveform_refresh_batch_active` (`src/app/controller/state/runtime/mod.rs:250`) |
| 188 | `compute_browser_toolbar_sections` (`vendor/radiant/src/gui/native_shell/layout_adapter/controls/browser_toolbar.rs:27`) |
| 187 | `build_with_style_and_runtime` (`vendor/radiant/src/gui/native_shell/layout/mod.rs:117`) |
| 178 | `folder_browser_group` (`vendor/radiant/src/gui/native_shell/state/automation/sidebar.rs:153`) |
| 175 | `render_confirm_prompt` (`vendor/radiant/src/gui/native_shell/state/overlays/prompt.rs:6`) |
| 174 | `push_waveform_playhead_overlay` (`vendor/radiant/src/gui/native_shell/state/waveform_segments/overlay.rs:8`) |
| 172 | `request_repaint` (`vendor/radiant/src/gui_runtime/native_vello.rs:152`) |
| 169 | `emit_edit_fade_overlays` (`vendor/radiant/src/gui/native_shell/state/waveform_segments/fades.rs:9`) |

## Over file-size budget

| Lines | File |
| --- | --- |
| 1674 | `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` |
| 1133 | `vendor/radiant/src/app/hotkeys.rs` |
| 942 | `src/app/controller/library/wavs/browser_pipeline/tests.rs` |
| 856 | `vendor/radiant/src/app/actions/mod.rs` |
| 763 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` |
| 713 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` |
| 630 | `src/app/controller/jobs/messages.rs` |
| 622 | `src/app/controller/library/wavs/similar/resolve/repository.rs` |
| 610 | `src/app/controller/ui/file_ops.rs` |
| 587 | `src/app/controller/library/source_folders/actions/rename_move_delete.rs` |
| 580 | `src/app/controller/jobs/file_ops_types.rs` |
| 536 | `src/app_core/native_shell/tests/browser_cache.rs` |
| 533 | `vendor/radiant/src/gui/native_shell/state/tests/selection_states.rs` |
| 524 | `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` |
| 523 | `vendor/radiant/src/gui/native_shell/state/tests/overlay_controls/context_and_controls.rs` |
| 518 | `src/app/controller/library/browser_controller/helpers.rs` |
| 518 | `crates/sempal-library/src/sample_sources/db/mod.rs` |
| 506 | `src/app_core/native_shell/options_panel_projection.rs` |
| 503 | `src/app/controller/tests/source_async.rs` |
| 482 | `src/app/controller/library/source_folders/delete_recovery/journal.rs` |
| 481 | `vendor/radiant/src/gui_runtime/native_vello/scene_cache/signatures.rs` |
| 481 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages_tests.rs` |
| 478 | `src/app/controller/library/wavs/waveform_rendering/render_apply.rs` |
| 475 | `src/app/controller/playback/transport/selection.rs` |
| 469 | `src/app/controller/library/source_folders/delete_recovery/recovery/tests.rs` |
| 465 | `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/rendering/row_states.rs` |
| 465 | `vendor/radiant/src/gui/native_shell/state/frame_text_cache.rs` |
| 462 | `src/app/controller/jobs/dispatch.rs` |
| 461 | `src/app/controller/state/runtime/mod.rs` |
| 456 | `src/app/controller/playback/transport/looping.rs` |
| 448 | `src/app_core/native_bridge/tests/projection_cache/overlays.rs` |
| 441 | `vendor/radiant/src/gui_runtime/native_vello/runtime_events/pointer.rs` |
| 435 | `src/app/controller/library/wavs/browser_actions/focus_navigation.rs` |
| 433 | `vendor/radiant/src/gui/native_shell/state.rs` |
| 426 | `src/app/controller/ui/waveform_slide.rs` |
| 420 | `vendor/radiant/src/gui_runtime/native_vello/runtime_state.rs` |
| 418 | `src/app/controller/tests/browser_actions/focus_navigation/preview_focus.rs` |
| 416 | `vendor/radiant/src/gui/native_shell/state/hit_testing/browser.rs` |
| 413 | `src/app/controller/library/wavs/selection_ops.rs` |
| 412 | `vendor/radiant/src/app/browser.rs` |
| 411 | `vendor/radiant/src/gui_runtime/native_vello/text_edit.rs` |
| 411 | `src/app/controller/config.rs` |
| 410 | `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs` |
| 409 | `vendor/radiant/src/gui_runtime/native_vello.rs` |
| 408 | `src/app/controller/library/sources/hydration.rs` |
| 407 | `src/selection/range.rs` |
| 405 | `src/app/controller/gui_fixtures.rs` |
| 401 | `src/app_core/actions/catalog/kinds.rs` |

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
| 630 | `src/app/controller/jobs/messages.rs` |
| 610 | `src/app/controller/ui/file_ops.rs` |
| 580 | `src/app/controller/jobs/file_ops_types.rs` |
| 524 | `src/app/controller/library/wavs/browser_pipeline/visible_rows.rs` |
| 518 | `src/app/controller/library/browser_controller/helpers.rs` |
| 481 | `vendor/radiant/src/gui_runtime/native_vello/scene_cache/signatures.rs` |
| 462 | `src/app/controller/jobs/dispatch.rs` |
| 420 | `vendor/radiant/src/gui_runtime/native_vello/runtime_state.rs` |
| 413 | `src/app/controller/library/wavs/selection_ops.rs` |
| 412 | `vendor/radiant/src/app/browser.rs` |
| 410 | `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs` |
| 405 | `src/app/controller/gui_fixtures.rs` |
| 401 | `src/app_core/actions/catalog/kinds.rs` |
| 396 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 395 | `src/app/controller/library/wavs/browser_facade.rs` |
| 392 | `src/app/controller/library/selection_edits/controller_actions.rs` |
| 388 | `vendor/radiant/src/gui_runtime/native_vello/text_runtime.rs` |
| 383 | `src/app_core/native_shell/browser_projection/row_window.rs` |
| 381 | `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs` |
| 376 | `vendor/radiant/src/app/sources.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
