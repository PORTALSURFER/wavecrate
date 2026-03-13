# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-03-13T13:52:34Z`
- Branch: `next`
- Commit: `8e157a1b`
- Rust files scanned: 730
- File-size budget limit: `400`

## Summary

- Over file-size budget: 41
- Function spans captured: 5284
- Files with `dead_code` suppressions: 0
- Files with `clippy::too_many_arguments` suppressions: 0
- Likely large-file test-gap hotspots (heuristic): 196

## Largest Rust files

| Lines | File |
| ---: | --- |
| 671 | `src/app_core/native_bridge/tests/projection_cache.rs` |
| 561 | `tests/unit/source_db_mod_tests.rs` |
| 550 | `apps/updater-helper/src/ui.rs` |
| 543 | `src/app/controller/library/analysis_jobs/enqueue/tests.rs` |
| 541 | `src/waveform/zoom_cache.rs` |
| 532 | `src/app/controller/tests/browser_core.rs` |
| 524 | `src/waveform/render/paint/lines.rs` |
| 523 | `src/app_core/controller/tests.rs` |
| 516 | `src/app/controller/tests/focus_random.rs` |
| 481 | `src/gui_test/aiv/packs/cases.rs` |
| 479 | `src/updater/mod.rs` |
| 474 | `src/app_core/native_bridge/projection_cache.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 471 | `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs` |
| 470 | `apps/installer/src/ui.rs` |
| 469 | `src/updater/archive.rs` |
| 467 | `src/wav_sanitize.rs` |
| 463 | `src/app_core/actions/catalog.rs` |
| 462 | `src/analysis/frequency_domain/stft.rs` |
| 456 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| ---: | --- |
| 285 | `run_folder_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs:16`) |
| 255 | `record_lookup` (`src/app_core/native_bridge/projection_cache.rs:56`) |
| 223 | `run_folder_sample_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs:62`) |
| 219 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 212 | `project_map_model` (`src/app_core/native_shell/map_projection.rs:11`) |
| 194 | `adjust_selected_rating` (`src/app/controller/playback/tagging.rs:231`) |
| 188 | `draw_line_aa` (`src/waveform/render/paint/lines.rs:312`) |
| 186 | `build_visible_rows` (`src/app/controller/library/wavs/browser_pipeline.rs:67`) |
| 181 | `apply_native_ui_action_routes_grouped_dispatch_cases` (`src/app_core/controller/tests.rs:41`) |
| 179 | `handle_analysis_message` (`src/app/controller/library/background_jobs/analysis.rs:8`) |
| 165 | `matches` (`src/app/controller/ui/hotkeys/types.rs:12`) |
| 164 | `handle_sample_drop_to_drop_target` (`src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:28`) |
| 156 | `handle_scan_finished` (`src/app/controller/library/background_jobs/scan.rs:21`) |
| 152 | `apply_waveform_native_ui_action` (`src/app_core/controller/waveform_actions.rs:12`) |
| 144 | `refresh_similarity_prep_progress` (`src/app/controller/library/similarity_prep/progress.rs:7`) |
| 144 | `handle_samples_drop_to_folder` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/plan.rs:29`) |
| 138 | `run_trash_move_task_with_progress` (`src/app/controller/library/trash_move.rs:50`) |
| 138 | `derive_waveform_projection_scalars` (`src/app_core/native_bridge/projection_cache/projection_key/waveform.rs:78`) |
| 137 | `ensure_feature_cache` (`src/app/controller/library/wavs/feature_cache.rs:73`) |
| 134 | `next` (`src/audio/decoder.rs:118`) |

## Over file-size budget

| Lines | File |
| ---: | --- |
| 671 | `src/app_core/native_bridge/tests/projection_cache.rs` |
| 561 | `tests/unit/source_db_mod_tests.rs` |
| 550 | `apps/updater-helper/src/ui.rs` |
| 543 | `src/app/controller/library/analysis_jobs/enqueue/tests.rs` |
| 541 | `src/waveform/zoom_cache.rs` |
| 532 | `src/app/controller/tests/browser_core.rs` |
| 524 | `src/waveform/render/paint/lines.rs` |
| 523 | `src/app_core/controller/tests.rs` |
| 516 | `src/app/controller/tests/focus_random.rs` |
| 481 | `src/gui_test/aiv/packs/cases.rs` |
| 479 | `src/updater/mod.rs` |
| 474 | `src/app_core/native_bridge/projection_cache.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 471 | `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs` |
| 470 | `apps/installer/src/ui.rs` |
| 469 | `src/updater/archive.rs` |
| 467 | `src/wav_sanitize.rs` |
| 463 | `src/app_core/actions/catalog.rs` |
| 462 | `src/analysis/frequency_domain/stft.rs` |
| 456 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |
| 455 | `src/app/controller/ui/hotkeys/actions.rs` |
| 446 | `src/app/controller/playback/mod.rs` |
| 442 | `src/app/controller/library/source_folders/tree.rs` |
| 433 | `src/app/controller/library/wavs/browser_lists.rs` |
| 432 | `src/external_drag.rs` |
| 432 | `src/waveform/model.rs` |
| 428 | `src/app/controller/ui/interaction_options.rs` |
| 426 | `src/app/state/browser.rs` |
| 424 | `src/app/controller/playback/tagging.rs` |
| 423 | `tools/analysis-admin/src/bin/sempal-hdbscan.rs` |
| 422 | `src/analysis/ann_index_tests.rs` |
| 422 | `src/app/controller/library/sources.rs` |
| 419 | `src/app/controller/library/wavs/browser_search.rs` |
| 418 | `src/app/controller/library/analysis_jobs/pool/job_progress.rs` |
| 418 | `src/app/controller/tests/waveform_nav_cursor.rs` |
| 418 | `src/sample_sources/db/write.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 406 | `src/app/controller/undo.rs` |
| 404 | `src/app/controller/playback/transport/selection.rs` |
| 403 | `src/app/controller/playback/loop_crossfade.rs` |
| 402 | `src/app/controller/library/browser_controller/actions.rs` |

## dead_code suppression density

None.

## too_many_arguments suppression density

None.

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.
Skips dedicated test modules/paths (`tests/**`, `tests.rs`, `*_test.rs`).

| Lines | File |
| ---: | --- |
| 550 | `apps/updater-helper/src/ui.rs` |
| 541 | `src/waveform/zoom_cache.rs` |
| 524 | `src/waveform/render/paint/lines.rs` |
| 481 | `src/gui_test/aiv/packs/cases.rs` |
| 479 | `src/updater/mod.rs` |
| 474 | `src/app_core/native_bridge/projection_cache.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 471 | `src/app/controller/library/analysis_jobs/pool/job_claim/compute_worker.rs` |
| 470 | `apps/installer/src/ui.rs` |
| 469 | `src/updater/archive.rs` |
| 467 | `src/wav_sanitize.rs` |
| 463 | `src/app_core/actions/catalog.rs` |
| 462 | `src/analysis/frequency_domain/stft.rs` |
| 456 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |
| 455 | `src/app/controller/ui/hotkeys/actions.rs` |
| 446 | `src/app/controller/playback/mod.rs` |
| 442 | `src/app/controller/library/source_folders/tree.rs` |
| 433 | `src/app/controller/library/wavs/browser_lists.rs` |
| 432 | `src/external_drag.rs` |
| 432 | `src/waveform/model.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
