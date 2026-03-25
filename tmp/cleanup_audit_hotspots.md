# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-03-25T15:35:03Z`
- Branch: `next`
- Commit: `efd1bbbd`
- Rust files scanned: 902
- File-size budget limit: `400`

## Summary

- Over file-size budget: 17
- Function spans captured: 5851
- Files with `dead_code` suppressions: 1
- Files with `clippy::too_many_arguments` suppressions: 0
- Likely large-file test-gap hotspots (heuristic): 86

## Largest Rust files

| Lines | File |
| --- | --- |
| 523 | `src/app/controller/tests/drag_drop_drop_targets.rs` |
| 504 | `src/app/controller/library/selection_export/background.rs` |
| 501 | `src/app/controller/library/selection_export.rs` |
| 501 | `src/app/controller/history.rs` |
| 496 | `src/app/controller/playback/transport/selection.rs` |
| 488 | `src/app/controller/library/selection_export/selection_export_tests.rs` |
| 476 | `src/app/controller/playback/tests.rs` |
| 459 | `src/app_core/controller.rs` |
| 459 | `src/app_core/actions/catalog/kinds.rs` |
| 437 | `src/app/controller/playback/audio_loader/stages.rs` |
| 432 | `src/app/controller/playback/recording/waveform_loader/tests.rs` |
| 422 | `src/app/controller/tests/waveform_cache_loading.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 409 | `src/app/controller/tests/browser_selection.rs` |
| 407 | `src/selection/range.rs` |
| 401 | `src/app_core/native_bridge/tests/bridge_runtime.rs` |
| 399 | `tests/unit/audio_output_tests.rs` |
| 398 | `src/sample_sources/db/file_ops_journal/tests.rs` |
| 397 | `src/sqlite_ext.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| --- | --- |
| 220 | `apply_waveform_native_ui_action` (`src/app_core/controller/waveform_actions.rs:12`) |
| 219 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 219 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests/dispatch.rs:49`) |
| 201 | `representative_action_for_kind` (`src/app_core/actions/catalog/entries.rs:46`) |
| 173 | `apply_browser_native_ui_action` (`src/app_core/controller.rs:225`) |
| 144 | `handle_samples_drop_to_folder` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs:29`) |
| 144 | `refresh_similarity_prep_progress` (`src/app/controller/library/similarity_prep/progress.rs:7`) |
| 138 | `derive_waveform_projection_scalars` (`src/app_core/native_bridge/projection_cache/projection_key/waveform.rs:78`) |
| 138 | `run_trash_move_task_with_progress` (`src/app/controller/library/trash_move.rs:50`) |
| 137 | `ensure_feature_cache` (`src/app/controller/library/wavs/feature_cache.rs:73`) |
| 134 | `next` (`src/audio/decoder.rs:118`) |
| 134 | `handle_waveform_command` (`src/app/controller/ui/hotkeys_controller/waveform/mod.rs:9`) |
| 132 | `run_undo_file_job` (`src/app/controller/undo_jobs.rs:13`) |
| 131 | `apply_configuration` (`src/app/controller/config.rs:14`) |
| 128 | `select_wav_known_index_with_options` (`src/app/controller/library/wavs/selection_ops.rs:113`) |
| 122 | `open_output_stream` (`src/audio/output/stream.rs:186`) |
| 122 | `capture` (`src/app_core/native_bridge/metrics/snapshot.rs:72`) |
| 122 | `handle_waveform_sample_drop_to_browser` (`src/app/controller/ui/drag_drop_controller/drag_effects/waveform_drop.rs:9`) |
| 120 | `apply_schema` (`src/sample_sources/library/schema_defs.rs:22`) |
| 111 | `request_id` (`src/app/controller/jobs/selection_export_types.rs:141`) |

## Over file-size budget

| Lines | File |
| --- | --- |
| 523 | `src/app/controller/tests/drag_drop_drop_targets.rs` |
| 504 | `src/app/controller/library/selection_export/background.rs` |
| 501 | `src/app/controller/library/selection_export.rs` |
| 501 | `src/app/controller/history.rs` |
| 496 | `src/app/controller/playback/transport/selection.rs` |
| 488 | `src/app/controller/library/selection_export/selection_export_tests.rs` |
| 476 | `src/app/controller/playback/tests.rs` |
| 459 | `src/app_core/controller.rs` |
| 459 | `src/app_core/actions/catalog/kinds.rs` |
| 437 | `src/app/controller/playback/audio_loader/stages.rs` |
| 432 | `src/app/controller/playback/recording/waveform_loader/tests.rs` |
| 422 | `src/app/controller/tests/waveform_cache_loading.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 409 | `src/app/controller/tests/browser_selection.rs` |
| 407 | `src/selection/range.rs` |
| 401 | `src/app_core/native_bridge/tests/bridge_runtime.rs` |

## dead_code suppression density

| Occurrences | File |
| --- | --- |
| 1 | `src/lib.rs` |

## too_many_arguments suppression density

None.

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.
Skips dedicated test modules/paths (`tests/**`, `tests.rs`, `*_test.rs`).

| Lines | File |
| --- | --- |
| 504 | `src/app/controller/library/selection_export/background.rs` |
| 501 | `src/app/controller/history.rs` |
| 488 | `src/app/controller/library/selection_export/selection_export_tests.rs` |
| 459 | `src/app_core/actions/catalog/kinds.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 407 | `src/selection/range.rs` |
| 391 | `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets/worker.rs` |
| 386 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 384 | `src/app/controller/jobs/file_ops_types.rs` |
| 348 | `src/app/controller/gui_fixtures.rs` |
| 343 | `src/external_drag/platform.rs` |
| 339 | `src/gui_test/aiv/packs/cases/browser.rs` |
| 339 | `src/app/controller/library/wavs/audio_loading.rs` |
| 339 | `src/app/controller/library/background_jobs/polling/library_handlers.rs` |
| 326 | `tools/bench-cli/src/bench/gui/rebuild_probe.rs` |
| 325 | `src/app/controller/library/wavs/entry_mutation.rs` |
| 323 | `src/app/controller/library/analysis_jobs/enqueue/invalidate.rs` |
| 322 | `src/app/controller/library/browser_controller/helpers.rs` |
| 320 | `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs` |
| 320 | `src/app/controller/playback/player/playback_start.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
