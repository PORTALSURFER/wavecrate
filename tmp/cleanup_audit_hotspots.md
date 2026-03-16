# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-03-16T15:02:23Z`
- Branch: `next`
- Commit: `5e165afe`
- Rust files scanned: 878
- File-size budget limit: `400`

## Summary

- Over file-size budget: 6
- Function spans captured: 5510
- Files with `dead_code` suppressions: 1
- Files with `clippy::too_many_arguments` suppressions: 1
- Likely large-file test-gap hotspots (heuristic): 81

## Largest Rust files

| Lines | File |
| --- | --- |
| 428 | `src/app/controller/library/background_jobs/scan.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 409 | `src/app/controller/tests/browser_selection.rs` |
| 409 | `src/app/controller/playback/tests.rs` |
| 407 | `src/selection/range.rs` |
| 399 | `tests/unit/audio_output_tests.rs` |
| 397 | `src/sqlite_ext.rs` |
| 394 | `src/app/controller/state/runtime.rs` |
| 394 | `src/app/controller/playback/transport/selection.rs` |
| 394 | `src/app/controller/library/selection_edits/ops.rs` |
| 393 | `src/issue_gateway/token_store/tests.rs` |
| 388 | `src/wav_sanitize.rs` |
| 387 | `src/audio/output/stream.rs` |
| 386 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 386 | `src/sample_sources/db/mod.rs` |
| 384 | `src/sample_sources/db/file_ops_journal/tests.rs` |
| 383 | `src/issue_gateway/api.rs` |
| 380 | `src/app/controller/playback/persistent_waveform_cache.rs` |
| 378 | `src/app/controller/tests/waveform_nav_render.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| --- | --- |
| 285 | `run_folder_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs:16`) |
| 223 | `run_folder_sample_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs:62`) |
| 219 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 219 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests/dispatch.rs:31`) |
| 190 | `handle_analysis_message` (`src/app/controller/library/background_jobs/analysis.rs:8`) |
| 173 | `handle_scan_finished` (`src/app/controller/library/background_jobs/scan.rs:21`) |
| 165 | `matches` (`src/app/controller/ui/hotkeys/types.rs:12`) |
| 164 | `handle_sample_drop_to_drop_target` (`src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:28`) |
| 158 | `apply_waveform_native_ui_action` (`src/app_core/controller/waveform_actions.rs:12`) |
| 144 | `handle_samples_drop_to_folder` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs:29`) |
| 144 | `refresh_similarity_prep_progress` (`src/app/controller/library/similarity_prep/progress.rs:7`) |
| 138 | `derive_waveform_projection_scalars` (`src/app_core/native_bridge/projection_cache/projection_key/waveform.rs:78`) |
| 138 | `run_trash_move_task_with_progress` (`src/app/controller/library/trash_move.rs:50`) |
| 137 | `ensure_feature_cache` (`src/app/controller/library/wavs/feature_cache.rs:73`) |
| 134 | `next` (`src/audio/decoder.rs:118`) |
| 132 | `run_undo_file_job` (`src/app/controller/undo_jobs.rs:13`) |
| 131 | `apply_configuration` (`src/app/controller/config.rs:14`) |
| 130 | `build_symphonia_peaks` (`src/waveform/decode/symphonia_reader.rs:64`) |
| 128 | `representative_action_for_kind` (`src/app_core/actions/catalog/entries.rs:39`) |
| 128 | `select_wav_known_index_with_options` (`src/app/controller/library/wavs/selection_ops.rs:113`) |

## Over file-size budget

| Lines | File |
| --- | --- |
| 428 | `src/app/controller/library/background_jobs/scan.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 409 | `src/app/controller/tests/browser_selection.rs` |
| 409 | `src/app/controller/playback/tests.rs` |
| 407 | `src/selection/range.rs` |

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
| 422 | `src/analysis/ann_index_tests.rs` |
| 407 | `src/selection/range.rs` |
| 386 | `tools/bench-cli/src/bench/gui/interactions.rs` |
| 373 | `src/app/controller/library/wavs/browser_actions/selection.rs` |
| 367 | `src/app/controller/playback/audio_loader/stages.rs` |
| 354 | `src/app/controller/playback/recording/waveform_loader/aggregation.rs` |
| 351 | `src/app/controller/playback/audio_options/controller.rs` |
| 348 | `src/app_core/native_bridge/metrics.rs` |
| 348 | `src/app/controller/gui_fixtures.rs` |
| 345 | `src/app_core/actions/catalog/kinds.rs` |
| 343 | `src/external_drag/platform.rs` |
| 339 | `src/gui_test/aiv/packs/cases/browser.rs` |
| 339 | `src/app/controller/library/wavs/audio_loading.rs` |
| 331 | `src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs` |
| 326 | `tools/bench-cli/src/bench/gui/rebuild_probe.rs` |
| 323 | `src/app/controller/library/analysis_jobs/enqueue/invalidate.rs` |
| 319 | `src/app/controller/library/analysis_jobs/db/artifacts.rs` |
| 307 | `src/app/controller/library/analysis_jobs/enqueue/enqueue_samples.rs` |
| 301 | `src/waveform/transients/peaks.rs` |
| 301 | `src/app/controller/playback/waveform_action_tests.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
