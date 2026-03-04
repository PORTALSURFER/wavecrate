# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-03-04T15:06:19Z`
- Branch: `next`
- Commit: `c3c2ab29`
- Rust files scanned: 523
- File-size budget limit: `400`

## Summary

- Over file-size budget: 56
- Function spans captured: 4494
- Files with `dead_code` suppressions: 31
- Files with `clippy::too_many_arguments` suppressions: 4
- Likely large-file test-gap hotspots (heuristic): 63

## Largest Rust files

| Lines | File |
| ---: | --- |
| 1025 | `src/app_core/native_bridge/tests.rs` |
| 861 | `src/app_core/native_shell/tests.rs` |
| 834 | `src/app_core/native_bridge/metrics.rs` |
| 758 | `src/app_core/native_shell.rs` |
| 708 | `src/app/controller/playback/transport.rs` |
| 695 | `src/app_core/native_bridge.rs` |
| 693 | `src/app/controller/playback/audio_loader.rs` |
| 688 | `src/audio/output.rs` |
| 687 | `src/app/controller/library/wavs/browser_actions.rs` |
| 686 | `src/app/controller/library/source_folders/delete_recovery.rs` |
| 683 | `src/app/controller/tests/browser_actions.rs` |
| 676 | `src/app/controller/library/background_jobs/polling.rs` |
| 660 | `src/app/controller/library/analysis_jobs/db/tests.rs` |
| 650 | `src/app/controller/tests/folders_core.rs` |
| 638 | `src/app/controller/library/wavs.rs` |
| 615 | `src/sample_sources/db/file_ops_journal.rs` |
| 607 | `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs` |
| 566 | `src/analysis/audio/normalize.rs` |
| 564 | `src/app/controller/library/analysis_jobs/pool/job_execution/backfill.rs` |
| 561 | `src/bin/sempal-updater/ui.rs` |

## Largest function spans (heuristic)

| Span (lines) | Function |
| ---: | --- |
| 314 | `sum_sq_serial` (`src/analysis/audio/normalize.rs:166`) |
| 285 | `run_folder_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker/folder_move_task.rs:16`) |
| 245 | `load_recording_waveform` (`src/app/controller/playback/recording/waveform_loader.rs:49`) |
| 240 | `spawn_compute_worker` (`src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs:163`) |
| 235 | `run_source_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs:297`) |
| 223 | `run_folder_sample_move_task` (`src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs:62`) |
| 219 | `settings_round_trip_preserves_fields` (`src/sample_sources/config_io/tests/save.rs:124`) |
| 212 | `run_clipboard_paste_job` (`src/app/controller/ui/clipboard_paste.rs:239`) |
| 206 | `project_map_model` (`src/app_core/native_shell/map_projection.rs:5`) |
| 192 | `record_lookup` (`src/app_core/native_bridge/projection_cache.rs:56`) |
| 186 | `build_visible_rows` (`src/app/controller/library/wavs/browser_pipeline.rs:67`) |
| 179 | `handle_analysis_message` (`src/app/controller/library/background_jobs/analysis.rs:8`) |
| 171 | `finish_active_drag` (`src/app/controller/ui/drag_drop_controller/actions.rs:164`) |
| 164 | `apply_schema_internal` (`src/sample_sources/db/schema.rs:21`) |
| 163 | `matches` (`src/app/controller/ui/hotkeys/types.rs:12`) |
| 156 | `handle_scan_finished` (`src/app/controller/library/background_jobs/scan.rs:21`) |
| 156 | `load_audio_inner` (`src/app/controller/playback/audio_loader.rs:368`) |
| 152 | `handle_sample_drop_to_drop_target` (`src/app/controller/ui/drag_drop_controller/drag_effects/drop_targets.rs:12`) |
| 151 | `draw_line_aa` (`src/waveform/render/paint/lines.rs:262`) |
| 144 | `refresh_similarity_prep_progress` (`src/app/controller/library/similarity_prep/progress.rs:7`) |

## Over file-size budget

| Lines | File |
| ---: | --- |
| 1025 | `src/app_core/native_bridge/tests.rs` |
| 861 | `src/app_core/native_shell/tests.rs` |
| 834 | `src/app_core/native_bridge/metrics.rs` |
| 758 | `src/app_core/native_shell.rs` |
| 708 | `src/app/controller/playback/transport.rs` |
| 695 | `src/app_core/native_bridge.rs` |
| 693 | `src/app/controller/playback/audio_loader.rs` |
| 688 | `src/audio/output.rs` |
| 687 | `src/app/controller/library/wavs/browser_actions.rs` |
| 686 | `src/app/controller/library/source_folders/delete_recovery.rs` |
| 683 | `src/app/controller/tests/browser_actions.rs` |
| 676 | `src/app/controller/library/background_jobs/polling.rs` |
| 660 | `src/app/controller/library/analysis_jobs/db/tests.rs` |
| 650 | `src/app/controller/tests/folders_core.rs` |
| 638 | `src/app/controller/library/wavs.rs` |
| 615 | `src/sample_sources/db/file_ops_journal.rs` |
| 607 | `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs` |
| 566 | `src/analysis/audio/normalize.rs` |
| 564 | `src/app/controller/library/analysis_jobs/pool/job_execution/backfill.rs` |
| 561 | `src/bin/sempal-updater/ui.rs` |
| 558 | `src/app/controller/tests/waveform.rs` |
| 556 | `src/app/controller/playback/recording/waveform_loader.rs` |
| 551 | `src/app/controller/library/selection_edits/mod.rs` |
| 551 | `src/audio/source.rs` |
| 543 | `src/app/controller/library/analysis_jobs/enqueue/tests.rs` |
| 541 | `src/waveform/zoom_cache.rs` |
| 531 | `src/app/controller/library/wavs/similar/resolve.rs` |
| 513 | `src/waveform/mod.rs` |
| 510 | `src/app/controller.rs` |
| 506 | `src/app/controller/library/wavs/waveform_loading.rs` |
| 503 | `src/analysis/ann_index/container.rs` |
| 501 | `src/waveform/render.rs` |
| 491 | `src/app/controller/ui/clipboard_paste.rs` |
| 490 | `src/bin/sempal-installer/ui.rs` |
| 486 | `src/app/controller/ui/drag_drop_controller/actions.rs` |
| 483 | `src/app/controller/ui/map_view.rs` |
| 481 | `src/updater/mod.rs` |
| 478 | `src/app/controller/tests/focus_random.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 469 | `src/updater/archive.rs` |
| 467 | `src/wav_sanitize.rs` |
| 442 | `src/app/controller/library/source_folders/tree.rs` |
| 441 | `src/app/controller/ui/hotkeys/actions.rs` |
| 440 | `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs` |
| 436 | `src/waveform/render/paint/lines.rs` |
| 431 | `src/app/controller/playback/player.rs` |
| 428 | `src/external_drag.rs` |
| 418 | `src/app/controller/library/analysis_jobs/pool/job_progress.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 412 | `src/analysis/frequency_domain/stft.rs` |
| 411 | `src/app/controller/library/sources.rs` |
| 404 | `src/app/controller/ui/hotkeys_controller/waveform.rs` |
| 402 | `src/app/controller/library/browser_controller/actions.rs` |
| 402 | `src/app/controller/playback/loop_crossfade.rs` |
| 402 | `src/app/state/browser.rs` |
| 401 | `src/sqlite_ext.rs` |

## dead_code suppression density

| Occurrences | File |
| ---: | --- |
| 8 | `vendor/radiant/src/gui/layout_core/model.rs` |
| 6 | `src/analysis/audio/silence.rs` |
| 5 | `src/sample_sources/library/tests.rs` |
| 4 | `vendor/radiant/src/gui/layout_core/engine/mod.rs` |
| 3 | `src/bin/sempal-installer/install.rs` |
| 3 | `vendor/radiant/src/gui/native_shell/state.rs` |
| 2 | `src/analysis/audio/decode_io.rs` |
| 2 | `src/app/controller/jobs/messages.rs` |
| 2 | `src/waveform/decode/symphonia_reader.rs` |
| 2 | `src/waveform/decode/wav_reader.rs` |
| 2 | `vendor/radiant/src/gui/native_shell/layout.rs` |
| 1 | `src/analysis/audio/mod.rs` |
| 1 | `src/app/controller/library/analysis_jobs/db/artifacts.rs` |
| 1 | `src/app/controller/library/analysis_jobs/pool/mod.rs` |
| 1 | `src/app/controller/library/source_folders/selection/filter.rs` |
| 1 | `src/app/controller/library/source_folders/selection/ops.rs` |
| 1 | `src/app/controller/state/cache.rs` |
| 1 | `src/audio/player/playback.rs` |
| 1 | `src/lib.rs` |
| 1 | `src/sample_sources/scanner/scan/runner.rs` |

## too_many_arguments suppression density

| Occurrences | File |
| ---: | --- |
| 2 | `vendor/radiant/src/gui/layout_core/engine/layout.rs` |
| 1 | `src/app/controller/ui/loading.rs` |
| 1 | `vendor/radiant/src/app/mod.rs` |
| 1 | `vendor/radiant/src/gui/layout_core/engine/layout/scroll_linear.rs` |

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.
Skips dedicated test modules/paths (`tests/**`, `tests.rs`, `*_test.rs`).

| Lines | File |
| ---: | --- |
| 834 | `src/app_core/native_bridge/metrics.rs` |
| 506 | `src/app/controller/library/wavs/waveform_loading.rs` |
| 483 | `src/app/controller/ui/map_view.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 441 | `src/app/controller/ui/hotkeys/actions.rs` |
| 436 | `src/waveform/render/paint/lines.rs` |
| 428 | `src/external_drag.rs` |
| 411 | `src/app/controller/library/sources.rs` |
| 402 | `src/app/controller/library/browser_controller/actions.rs` |
| 399 | `src/app_core/native_shell/browser_projection.rs` |
| 398 | `src/app/controller/library/wavs/browser_search_worker/pipeline/stages.rs` |
| 398 | `src/selection/range.rs` |
| 397 | `src/bin/sempal-hdbscan.rs` |
| 397 | `src/sample_sources/db/schema.rs` |
| 396 | `src/app/controller/library/source_folders/selection/ops.rs` |
| 393 | `src/sample_sources/db/read.rs` |
| 387 | `src/app/controller/library/wavs/browser_pipeline.rs` |
| 385 | `src/bin/bench/gui/interactions.rs` |
| 383 | `src/app/controller/library/wavs/waveform_rendering.rs` |
| 373 | `src/sample_sources/db/write.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
