# Cleanup Hotspot Audit Snapshot

- Generated (UTC): `2026-02-27T21:51:44Z`
- Branch: `next`
- Commit: `e8c0d533`
- Rust files scanned: 493
- File-size budget limit: `400`

## Summary

- Over file-size budget: 64
- Files with `dead_code` suppressions: 34
- Files with `clippy::too_many_arguments` suppressions: 4
- Likely large-file test-gap hotspots (heuristic): 63

## Largest Rust files

| Lines | File |
| ---: | --- |
| 566 | `src/analysis/audio/normalize.rs` |
| 503 | `src/analysis/ann_index/container.rs` |
| 336 | `src/analysis/ann_index_tests.rs` |
| 299 | `src/analysis/ann_index/mod.rs` |
| 289 | `src/analysis/ann_index/build.rs` |
| 230 | `src/analysis/audio/decode_io.rs` |
| 216 | `src/analysis/audio/silence.rs` |
| 214 | `src/analysis/audio/analysis_prep.rs` |
| 153 | `src/analysis/ann_index/storage.rs` |
| 130 | `src/analysis/ann_index/update.rs` |
| 113 | `src/analysis/audio_decode.rs` |
| 107 | `benches/ann_index.rs` |
| 99 | `build.rs` |
| 94 | `src/analysis/fft.rs` |
| 59 | `src/analysis/ann_index/state.rs` |
| 56 | `src/analysis/audio/resample.rs` |
| 52 | `benches/tagging.rs` |
| 47 | `src/analysis/audio/mod.rs` |
| 26 | `src/analysis/features.rs` |
| 5 | `src/analysis/audio/decode.rs` |

## Over file-size budget

| Lines | File |
| ---: | --- |
| 1171 | `src/app/controller/jobs.rs` |
| 1085 | `src/issue_gateway/token_store.rs` |
| 983 | `src/audio/output.rs` |
| 957 | `src/app_core/native_bridge/projection_cache.rs` |
| 923 | `src/app/controller/library/wavs.rs` |
| 889 | `src/selection.rs` |
| 819 | `src/app_core/native_bridge/tests.rs` |
| 797 | `src/app/controller/playback/mod.rs` |
| 791 | `src/app_core/native_bridge/metrics.rs` |
| 774 | `src/sample_sources/db/mod.rs` |
| 768 | `src/app_core/native_shell/tests.rs` |
| 743 | `src/app_core/native_shell.rs` |
| 708 | `src/app/controller/playback/transport.rs` |
| 697 | `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs` |
| 686 | `src/app/controller/library/source_folders/delete_recovery.rs` |
| 683 | `src/app/controller/tests/browser_actions.rs` |
| 675 | `src/app/controller/library/wavs/browser_actions.rs` |
| 669 | `src/app_core/native_bridge.rs` |
| 660 | `src/app/controller/library/analysis_jobs/db/tests.rs` |
| 650 | `src/app/controller/tests/folders_core.rs` |
| 615 | `src/sample_sources/db/file_ops_journal.rs` |
| 589 | `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs` |
| 566 | `src/analysis/audio/normalize.rs` |
| 564 | `src/app/controller/library/analysis_jobs/pool/job_execution/backfill.rs` |
| 559 | `src/app/controller/playback/audio_loader.rs` |
| 558 | `src/app/controller/tests/waveform.rs` |
| 556 | `src/app/controller/playback/recording/waveform_loader.rs` |
| 553 | `src/bin/sempal-updater/ui.rs` |
| 551 | `src/app/controller/library/selection_edits/mod.rs` |
| 551 | `src/audio/source.rs` |
| 543 | `src/app/controller/library/analysis_jobs/enqueue/tests.rs` |
| 531 | `src/app/controller/library/wavs/similar/resolve.rs` |
| 513 | `src/waveform/mod.rs` |
| 506 | `src/app/controller/library/wavs/waveform_loading.rs` |
| 504 | `src/app/controller.rs` |
| 503 | `src/analysis/ann_index/container.rs` |
| 501 | `src/waveform/render.rs` |
| 498 | `src/waveform/zoom_cache.rs` |
| 491 | `src/app/controller/ui/clipboard_paste.rs` |
| 487 | `src/app/controller/library/background_jobs/mod.rs` |
| 486 | `src/app/controller/ui/drag_drop_controller/actions.rs` |
| 481 | `src/bin/sempal-installer/ui.rs` |
| 481 | `src/updater/mod.rs` |
| 478 | `src/app/controller/tests/focus_random.rs` |
| 475 | `src/app/controller/ui/map_view.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 469 | `src/updater/archive.rs` |
| 467 | `src/wav_sanitize.rs` |
| 442 | `src/app/controller/library/source_folders/tree.rs` |
| 441 | `src/app/controller/ui/hotkeys/actions.rs` |
| 440 | `src/app/controller/library/analysis_jobs/pool/job_claim/mod.rs` |
| 436 | `src/waveform/render/paint/lines.rs` |
| 428 | `src/external_drag.rs` |
| 424 | `src/app/controller/playback/player.rs` |
| 418 | `src/app/controller/library/analysis_jobs/pool/job_progress.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 412 | `src/analysis/frequency_domain/stft.rs` |
| 409 | `src/app/controller/library/sources.rs` |
| 409 | `src/app/controller/state/runtime.rs` |
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
| 4 | `src/app/controller/ui/hotkeys/types.rs` |
| 4 | `src/sample_sources/mod.rs` |
| 4 | `vendor/radiant/src/gui/layout_core/engine/mod.rs` |
| 3 | `src/bin/sempal-installer/install.rs` |
| 3 | `vendor/radiant/src/gui/native_shell/state.rs` |
| 2 | `src/analysis/audio/decode_io.rs` |
| 2 | `src/app/controller/jobs.rs` |
| 2 | `src/sample_sources/db/write.rs` |
| 2 | `src/waveform/decode/symphonia_reader.rs` |
| 2 | `src/waveform/decode/wav_reader.rs` |
| 2 | `vendor/radiant/src/gui/native_shell/layout.rs` |
| 1 | `src/analysis/audio/mod.rs` |
| 1 | `src/app/controller/library/analysis_jobs/db/artifacts.rs` |
| 1 | `src/app/controller/library/analysis_jobs/pool/mod.rs` |
| 1 | `src/app/controller/library/source_folders/selection/filter.rs` |
| 1 | `src/app/controller/library/source_folders/selection/ops.rs` |
| 1 | `src/app/controller/state/cache.rs` |

## too_many_arguments suppression density

| Occurrences | File |
| ---: | --- |
| 2 | `vendor/radiant/src/gui/layout_core/engine/layout.rs` |
| 1 | `src/app_core/native_bridge/projection_cache.rs` |
| 1 | `vendor/radiant/src/app/mod.rs` |
| 1 | `vendor/radiant/src/gui/layout_core/engine/layout/scroll_linear.rs` |

## Likely test-gap hotspots (heuristic)

Files with at least `200` lines and no local `#[cfg(test)]` or `mod tests` marker.

| Lines | File |
| ---: | --- |
| 819 | `src/app_core/native_bridge/tests.rs` |
| 791 | `src/app_core/native_bridge/metrics.rs` |
| 768 | `src/app_core/native_shell/tests.rs` |
| 660 | `src/app/controller/library/analysis_jobs/db/tests.rs` |
| 589 | `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/worker.rs` |
| 543 | `src/app/controller/library/analysis_jobs/enqueue/tests.rs` |
| 506 | `src/app/controller/library/wavs/waveform_loading.rs` |
| 487 | `src/app/controller/library/background_jobs/mod.rs` |
| 475 | `src/app/controller/ui/map_view.rs` |
| 473 | `src/app/controller/playback/audio_options.rs` |
| 441 | `src/app/controller/ui/hotkeys/actions.rs` |
| 436 | `src/waveform/render/paint/lines.rs` |
| 428 | `src/external_drag.rs` |
| 413 | `src/sample_sources/scanner/scan/tests.rs` |
| 409 | `src/app/controller/library/sources.rs` |
| 402 | `src/app/controller/library/browser_controller/actions.rs` |
| 397 | `src/bin/sempal-hdbscan.rs` |
| 397 | `src/sample_sources/db/schema.rs` |
| 396 | `src/app/controller/library/source_folders/selection/ops.rs` |
| 393 | `src/sample_sources/db/read.rs` |

## Suggested follow-up

1. Triage top over-budget files and plan behavior-preserving splits.
2. Remove or test-gate high-density suppressions after each refactor slice.
3. Add focused tests for top heuristic gaps where behavior is non-trivial.
