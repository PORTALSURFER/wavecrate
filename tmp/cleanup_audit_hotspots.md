# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-03-29T16:56:06Z`
- Branch: `next`
- Commit: `4c94dac5`
- Rust files scanned: 1254
- File-size budget limit: `400`

## Summary

- Over file-size budget: 58
- Function spans captured: 8409
- Files with `dead_code` suppressions: 2
- Files with `clippy::too_many_arguments` suppressions: 3
- Likely large-file test-gap hotspots (heuristic): 142

## Largest Rust files

| Lines | File |
| --- | --- |
| 1636 | `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` |
| 875 | `vendor/radiant/src/app/hotkeys.rs` |
| 812 | `vendor/radiant/src/gui/native_shell/state/tests/sidebar.rs` |
| 763 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` |
| 738 | `vendor/radiant/src/app/actions/mod.rs` |
| 713 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` |
| 650 | `src/app_core/native_bridge/tests/bridge_runtime.rs` |
| 630 | `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome.rs` |
| 619 | `src/app/controller/tests/drag_drop_folders.rs` |
| 612 | `src/app/controller/library/background_jobs/polling/tests.rs` |
| 579 | `vendor/radiant/src/gui/native_shell/layout_adapter/sidebar_header.rs` |
| 571 | `vendor/radiant/src/gui_runtime/native_vello/tests/key_bindings.rs` |
| 557 | `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders.rs` |
| 556 | `vendor/radiant/src/gui/native_shell/state/tests/frame_build.rs` |
| 547 | `src/app/controller/tests/folders_core/rename_delete_recovery.rs` |
| 543 | `src/app_core/actions/catalog/kinds.rs` |
| 529 | `vendor/radiant/src/gui/native_shell/state/tests/chrome_layout/waveform_toolbar.rs` |
| 527 | `vendor/radiant/src/gui/native_shell/state/tests/waveform_selection.rs` |
| 521 | `src/app_core/controller/tests/waveform.rs` |
| 521 | `src/app/controller/library/slices/slices_tests.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| --- | --- |
| 594 | `is_active` (`vendor/radiant/src/app/hotkeys.rs:153`) |
| 279 | `apply_waveform_native_ui_action` (`src/app_core/controller/waveform_actions.rs:12`) |
| 275 | `waveform_toolbar_buttons` (`vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs:16`) |
| 262 | `redraw` (`vendor/radiant/src/gui_runtime/native_vello/runtime_render/present.rs:81`) |
| 262 | `render_browser_frame` (`vendor/radiant/src/gui/native_shell/state/frame_build/browser/panel.rs:3`) |
| 250 | `render_browser_rows_window` (`vendor/radiant/src/gui/native_shell/state/frame_build/browser/rows.rs:3`) |
| 238 | `waveform_drag_action_clamps_and_preserves_selection_anchor` (`vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs:478`) |
| 235 | `representative_action_for_kind` (`src/app_core/actions/catalog/entries.rs:46`) |
| 224 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 219 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests/dispatch.rs:49`) |
| 210 | `sizing_for_tier` (`vendor/radiant/src/gui/native_shell/style/sizing.rs:12`) |
| 209 | `apply_browser_native_ui_action` (`src/app_core/controller/browser_actions.rs:12`) |
| 202 | `build_chrome_motion_overlay_into` (`vendor/radiant/src/gui/native_shell/state/motion_overlay.rs:59`) |
| 197 | `action_slug` (`vendor/radiant/src/gui/native_shell/state/automation/helpers.rs:98`) |
| 185 | `build_with_style_and_runtime` (`vendor/radiant/src/gui/native_shell/layout/mod.rs:116`) |
| 180 | `push_waveform_playhead_overlay` (`vendor/radiant/src/gui/native_shell/state/waveform_segments/overlay.rs:8`) |
| 175 | `render_confirm_prompt` (`vendor/radiant/src/gui/native_shell/state/overlays/prompt.rs:6`) |
| 172 | `folder_browser_group` (`vendor/radiant/src/gui/native_shell/state/automation/sidebar.rs:121`) |
| 171 | `render_status_bar` (`vendor/radiant/src/gui/native_shell/state/frame_build/status_bar.rs:5`) |
| 169 | `emit_edit_fade_overlays` (`vendor/radiant/src/gui/native_shell/state/waveform_segments/fades.rs:9`) |

## Over file-size budget

| Lines | File |
| --- | --- |
| 1636 | `vendor/radiant/src/gui_runtime/native_vello/tests/queue_runtime.rs` |
| 875 | `vendor/radiant/src/app/hotkeys.rs` |
| 812 | `vendor/radiant/src/gui/native_shell/state/tests/sidebar.rs` |
| 763 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_drag_finish.rs` |
| 738 | `vendor/radiant/src/app/actions/mod.rs` |
| 713 | `vendor/radiant/src/gui_runtime/native_vello/tests/waveform_pointer/selection_creation.rs` |
| 650 | `src/app_core/native_bridge/tests/bridge_runtime.rs` |
| 630 | `vendor/radiant/src/gui/native_shell/state/hit_testing/chrome.rs` |
| 619 | `src/app/controller/tests/drag_drop_folders.rs` |
| 612 | `src/app/controller/library/background_jobs/polling/tests.rs` |
| 579 | `vendor/radiant/src/gui/native_shell/layout_adapter/sidebar_header.rs` |
| 571 | `vendor/radiant/src/gui_runtime/native_vello/tests/key_bindings.rs` |
| 557 | `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders.rs` |
| 556 | `vendor/radiant/src/gui/native_shell/state/tests/frame_build.rs` |
| 547 | `src/app/controller/tests/folders_core/rename_delete_recovery.rs` |
| 543 | `src/app_core/actions/catalog/kinds.rs` |
| 529 | `vendor/radiant/src/gui/native_shell/state/tests/chrome_layout/waveform_toolbar.rs` |
| 527 | `vendor/radiant/src/gui/native_shell/state/tests/waveform_selection.rs` |
| 521 | `src/app_core/controller/tests/waveform.rs` |
| 521 | `src/app/controller/library/slices/slices_tests.rs` |
| 519 | `src/app/controller/tests/browser_actions/row_actions.rs` |
| 498 | `vendor/radiant/src/gui/native_shell/state/tests/overlay_controls.rs` |
| 495 | `vendor/radiant/src/gui_runtime/native_vello/tests/runtime_core.rs` |
| 495 | `src/app/controller/playback/tests/waveform_actions.rs` |
| 489 | `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs` |
| 488 | `src/app_core/controller/tests/browser_sources.rs` |
| 475 | `vendor/radiant/src/gui/native_shell/state/hit_testing/waveform.rs` |
| 475 | `src/app/controller/playback/transport/selection.rs` |
| 475 | `src/app/controller/library/selection_export/selection_export_tests/waveform_selection_export_tests.rs` |
| 467 | `vendor/radiant/src/gui/native_shell/state/tests/browser_scrollbars.rs` |
| 457 | `vendor/radiant/src/gui/native_shell/layout_adapter/waveform_annotations.rs` |
| 457 | `src/app/controller/state/runtime.rs` |
| 448 | `src/gui_test/runner.rs` |
| 448 | `src/app/controller/library/wavs/entry_mutation.rs` |
| 444 | `src/analysis/audio/exact_duplicates.rs` |
| 432 | `src/app/controller/playback/transport/seek.rs` |
| 432 | `src/app/controller/playback/recording/waveform_loader/tests.rs` |
| 424 | `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_row_decor.rs` |
| 423 | `src/sample_sources/db/file_ops_journal/tests.rs` |
| 422 | `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs` |
| 422 | `src/app/controller/tests/waveform_cache_loading.rs` |
| 422 | `src/app/controller/history.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 419 | `src/app/controller/library/wavs/audio_loading.rs` |
| 419 | `src/app/controller/library/source_folders/delete_recovery/recovery.rs` |
| 413 | `vendor/radiant/src/gui/layout_core/engine/context.rs` |
| 412 | `vendor/radiant/src/gui_runtime/native_vello/tests/browser_pointer/surface_routes.rs` |
| 409 | `src/app/controller/tests/browser_selection.rs` |
| 407 | `vendor/radiant/src/gui/native_shell/state/automation.rs` |
| 407 | `src/selection/range.rs` |
| 406 | `vendor/radiant/src/gui/native_shell/state/tests/browser_rows/virtualization.rs` |
| 406 | `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves/worker.rs` |
| 406 | `src/app/controller/tests/folders_core/tree_projection.rs` |
| 404 | `src/sample_sources/scanner/scan/tests.rs` |
| 403 | `vendor/radiant/src/gui_runtime/native_vello/input.rs` |
| 403 | `vendor/radiant/src/gui/native_shell/state/tests/overlays/waveform_hover.rs` |
| 403 | `src/app/controller/library/wavs/similar/background.rs` |
| 402 | `src/app/controller/tests/browser_actions/focus_navigation.rs` |

## dead_code suppression density

| Occurrences | File |
| --- | --- |
| 8 | `vendor/radiant/src/gui/layout_core/model.rs` |
| 1 | `src/lib.rs` |

## too_many_arguments suppression density

| Occurrences | File |
| --- | --- |
| 2 | `vendor/radiant/src/gui/layout_core/engine/layout.rs` |
| 1 | `vendor/radiant/src/gui/layout_core/engine/layout/scroll_linear.rs` |
| 1 | `vendor/radiant/src/app/sources.rs` |

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.
Skips dedicated test modules/paths (`tests/**`, `tests.rs`, `*_test.rs`).

| Lines | File |
| --- | --- |
| 557 | `vendor/radiant/src/gui/native_shell/state/frame_build/chrome/sidebar_parts/folders.rs` |
| 543 | `src/app_core/actions/catalog/kinds.rs` |
| 521 | `src/app/controller/library/slices/slices_tests.rs` |
| 489 | `vendor/radiant/src/gui_runtime/native_vello/text_bpm.rs` |
| 475 | `src/app/controller/library/selection_export/selection_export_tests/waveform_selection_export_tests.rs` |
| 424 | `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/browser_row_decor.rs` |
| 422 | `vendor/radiant/src/gui/native_shell/state/frame_build/overlay/focus.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 419 | `src/app/controller/library/wavs/audio_loading.rs` |
| 413 | `vendor/radiant/src/gui/layout_core/engine/context.rs` |
| 407 | `src/selection/range.rs` |
| 403 | `src/app/controller/library/wavs/similar/background.rs` |
| 400 | `src/app/controller/playback/player/playback_start.rs` |
| 390 | `src/app/controller/jobs/file_ops_types.rs` |
| 388 | `vendor/radiant/src/gui/native_shell/state/toolbar_helpers/waveform_toolbar.rs` |
| 386 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 360 | `src/app_core/controller/waveform_actions.rs` |
| 357 | `vendor/radiant/src/gui/native_shell/state/waveform_segments/fades.rs` |
| 354 | `vendor/radiant/src/gui/layout_core/engine/virtualization_tests.rs` |
| 354 | `src/app_core/native_shell/waveform_projection.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
