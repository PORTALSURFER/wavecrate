# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-03-14T15:59:00Z`
- Branch: `next`
- Commit: `3ca6e903`
- Rust files scanned: 780
- File-size budget limit: `400`

## Summary

- Over file-size budget: 26
- Function spans captured: 5370
- Files with `dead_code` suppressions: 1
- Files with `clippy::too_many_arguments` suppressions: 1
- Likely large-file test-gap hotspots (heuristic): 84

## Largest Rust files

| Lines | File |
| --- | --- |
| 479 | `src/updater/mod.rs` |
| 474 | `src/app_core/native_bridge/projection_cache.rs` |
| 469 | `src/updater/archive.rs` |
| 468 | `src/app/controller/tests/playback_loop.rs` |
| 467 | `src/wav_sanitize.rs` |
| 464 | `src/app/controller/library/wavs/browser_search.rs` |
| 462 | `src/analysis/frequency_domain/stft.rs` |
| 456 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |
| 455 | `src/app/controller/ui/hotkeys/actions.rs` |
| 454 | `src/app/controller/playback/mod.rs` |
| 442 | `src/app/controller/library/source_folders/tree.rs` |
| 433 | `src/app/controller/library/wavs/browser_lists.rs` |
| 432 | `src/waveform/model.rs` |
| 428 | `src/app/controller/ui/interaction_options.rs` |
| 426 | `src/app/state/browser.rs` |
| 423 | `tools/analysis-admin/src/bin/sempal-hdbscan.rs` |
| 422 | `src/app/controller/library/sources.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 418 | `src/sample_sources/db/write.rs` |
| 418 | `src/app/controller/tests/waveform_nav_cursor.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| --- | --- |
| 285 | `run_folder_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs:16`) |
| 255 | `record_lookup` (`src/app_core/native_bridge/projection_cache.rs:56`) |
| 223 | `run_folder_sample_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs:62`) |
| 219 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 218 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests/dispatch.rs:31`) |
| 212 | `project_map_model` (`src/app_core/native_shell/map_projection.rs:11`) |
| 186 | `build_visible_rows` (`src/app/controller/library/wavs/browser_pipeline.rs:67`) |
| 179 | `handle_analysis_message` (`src/app/controller/library/background_jobs/analysis.rs:8`) |
| 165 | `matches` (`src/app/controller/ui/hotkeys/types.rs:12`) |
| 164 | `handle_sample_drop_to_drop_target` (`src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:28`) |
| 158 | `apply_waveform_native_ui_action` (`src/app_core/controller/waveform_actions.rs:12`) |
| 156 | `handle_scan_finished` (`src/app/controller/library/background_jobs/scan.rs:21`) |
| 144 | `handle_samples_drop_to_folder` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs:29`) |
| 144 | `refresh_similarity_prep_progress` (`src/app/controller/library/similarity_prep/progress.rs:7`) |
| 138 | `derive_waveform_projection_scalars` (`src/app_core/native_bridge/projection_cache/projection_key/waveform.rs:78`) |
| 138 | `run_trash_move_task_with_progress` (`src/app/controller/library/trash_move.rs:50`) |
| 137 | `ensure_feature_cache` (`src/app/controller/library/wavs/feature_cache.rs:73`) |
| 134 | `next` (`src/audio/decoder.rs:118`) |
| 132 | `run_undo_file_job` (`src/app/controller/undo_jobs.rs:13`) |
| 131 | `apply_configuration` (`src/app/controller/config.rs:14`) |

## Over file-size budget

| Lines | File |
| --- | --- |
| 479 | `src/updater/mod.rs` |
| 474 | `src/app_core/native_bridge/projection_cache.rs` |
| 469 | `src/updater/archive.rs` |
| 468 | `src/app/controller/tests/playback_loop.rs` |
| 467 | `src/wav_sanitize.rs` |
| 464 | `src/app/controller/library/wavs/browser_search.rs` |
| 462 | `src/analysis/frequency_domain/stft.rs` |
| 456 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |
| 455 | `src/app/controller/ui/hotkeys/actions.rs` |
| 454 | `src/app/controller/playback/mod.rs` |
| 442 | `src/app/controller/library/source_folders/tree.rs` |
| 433 | `src/app/controller/library/wavs/browser_lists.rs` |
| 432 | `src/waveform/model.rs` |
| 428 | `src/app/controller/ui/interaction_options.rs` |
| 426 | `src/app/state/browser.rs` |
| 423 | `tools/analysis-admin/src/bin/sempal-hdbscan.rs` |
| 422 | `src/app/controller/library/sources.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 418 | `src/sample_sources/db/write.rs` |
| 418 | `src/app/controller/tests/waveform_nav_cursor.rs` |
| 418 | `src/app/controller/library/analysis_jobs/pool/job_progress.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 409 | `src/app/controller/playback/tests.rs` |
| 406 | `src/app/controller/undo.rs` |
| 403 | `src/app/controller/playback/loop_crossfade.rs` |
| 402 | `src/app/controller/library/browser_controller/actions.rs` |

## dead_code suppression density

| Occurrences | File |
| --- | --- |
| 1 | `src/lib.rs` |

## too_many_arguments suppression density

| Occurrences | File |
| --- | --- |
| 1 | `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker/execution.rs` |

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.
Skips dedicated test modules/paths (`tests/**`, `tests.rs`, `*_test.rs`).

| Lines | File |
| --- | --- |
| 456 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |
| 455 | `src/app/controller/ui/hotkeys/actions.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 418 | `src/sample_sources/db/write.rs` |
| 402 | `src/app/controller/library/browser_controller/actions.rs` |
| 397 | `src/selection/range.rs` |
| 395 | `src/app/controller/library/source_folders/selection/ops.rs` |
| 387 | `src/app/controller/library/wavs/browser_pipeline.rs` |
| 386 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 384 | `src/app/controller/library/wavs/waveform_rendering.rs` |
| 367 | `src/sample_sources/db/read.rs` |
| 367 | `src/app/controller/playback/audio_loader/stages.rs` |
| 354 | `src/app/controller/playback/recording/waveform_loader/aggregation.rs` |
| 351 | `src/app/controller/playback/audio_options/controller.rs` |
| 348 | `src/app_core/native_bridge/metrics.rs` |
| 345 | `src/app_core/actions/catalog/kinds.rs` |
| 343 | `src/external_drag/platform.rs` |
| 339 | `src/app/controller/library/wavs/audio_loading.rs` |
| 331 | `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` |
| 326 | `tools/bench-cli/src/bench/gui/rebuild_probe.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
