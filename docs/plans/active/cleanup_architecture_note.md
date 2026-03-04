# Cleanup Architecture Note

Status: Active guidance for cleanup passes
Last updated (UTC): 2026-03-04

## Purpose

This note anchors cleanup work to explicit module boundaries so future refactors reduce debt without re-introducing cross-layer coupling. It complements `tmp/cleanup_plan.md` by defining *where* responsibilities belong.

## Boundary map

### UI composition and event wiring

- Ownership: `src/app/controller/ui/**`
- Allowed responsibilities:
  - UI state transitions and user input handling.
  - Translation of UI actions into controller-level commands.
- Keep out:
  - Direct database mutation logic.
  - Background job orchestration internals.
- Current hotspots:
  - `src/app/controller/ui/drag_drop_controller/drag_effects/source_moves.rs`
  - `src/app/controller/ui/drag_drop_controller/drag_effects/folder_moves/**`

### Library/domain orchestration

- Ownership: `src/app/controller/library/**`
- Allowed responsibilities:
  - Sample-browser domain workflows (selection, metadata cache updates, analysis enqueueing).
  - Coordination between UI intents and persistence/service layers.
- Keep out:
  - Native shell projection internals.
  - Low-level DB transaction primitives.
- Current hotspots:
  - `src/app/controller/library/wavs.rs`
  - `src/app/controller/library/background_jobs/mod.rs`

### Native bridge and projection cache

- Ownership: `src/app_core/native_bridge/**`, `src/app_core/native_shell/**`
- Allowed responsibilities:
  - Projection keys/materialization, retained cache policy, shell model projection.
  - Native-facing model translation and perf metrics collection.
- Keep out:
  - Browser/library business rules.
  - DB writes and filesystem orchestration.
- Current hotspots:
  - `src/app_core/native_bridge/projection_cache.rs`
  - `src/app_core/native_shell.rs`

### Persistence and file operation infrastructure

- Ownership: `src/sample_sources/**`
- Allowed responsibilities:
  - DB schema/read/write APIs.
  - Journal-backed file operations and recovery behavior.
- Keep out:
  - UI policy and rendering concerns.
- Current hotspots:
  - `src/sample_sources/db/file_ops_journal.rs`
  - `src/sample_sources/db/mod.rs`

### External integration and secret storage

- Ownership: `src/issue_gateway/**`
- Allowed responsibilities:
  - Gateway DTOs, auth token persistence, keyring/fallback storage integration.
- Keep out:
  - Controller job lifecycle state management.
- Current hotspots:
  - `src/issue_gateway/token_store.rs`

### Vendor rendering engine

- Ownership: `vendor/radiant/src/gui/native_shell/**`
- Allowed responsibilities:
  - Rendering and native-shell visual model composition inside `radiant`.
- Keep out:
  - App-specific controller policies from `sempal`.
- Current hotspots:
  - `vendor/radiant/src/gui/native_shell/state.rs`

## Cleanup guardrails

1. Refactors stay behavior-preserving unless the item explicitly calls for a bug fix.
2. Move complexity downward into focused helpers, not sideways into facades.
3. New cross-layer data should pass through typed DTO/projection boundaries, not ad-hoc maps/tuples.
4. Files trending above 400 LOC should split by responsibility before adding new features.
5. Non-trivial extracted logic gets focused unit tests in the owning module tree.

## Validation checklist for cleanup items

1. `bash scripts/ci_local.sh` is green before push.
2. `tmp/cleanup_plan.md` item is marked complete with date + commit hash.
3. `AGENTS.md` and `MEMORY.md` still point to current active plans.
4. No new `#[allow(dead_code)]` or `#[allow(clippy::too_many_arguments)]` added without rationale.
